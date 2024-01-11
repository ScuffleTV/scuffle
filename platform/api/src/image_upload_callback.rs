use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use async_nats::jetstream::AckKind;
use futures_util::StreamExt;
use pb::ext::UlidExt;
use pb::scuffle::platform::internal::events::ProcessedImage;
use pb::scuffle::platform::internal::types::{uploaded_file_metadata, UploadedFileMetadata};
use prost::Message;
use tokio::select;

use crate::config::ImageUploaderConfig;
use crate::database::UploadedFile;
use crate::global::ApiGlobal;
use crate::subscription::SubscriptionTopic;

const PROFILE_PICTURE_CONSUMER_NAME: &str = "profile-picture-consumer";

pub async fn run<G: ApiGlobal>(global: Arc<G>) -> anyhow::Result<()> {
	let config = global.config::<ImageUploaderConfig>();

	let profile_picture_stream = global
		.jetstream()
		.get_or_create_stream(async_nats::jetstream::stream::Config {
			name: config.profile_picture_callback_subject.clone(),
			subjects: vec![config.profile_picture_callback_subject.clone()],
			max_consumers: 1,
			..Default::default()
		})
		.await
		.context("failed to create profile picture stream")?;

	let profile_picture_consumer = profile_picture_stream
		.get_or_create_consumer(
			PROFILE_PICTURE_CONSUMER_NAME,
			async_nats::jetstream::consumer::pull::Config {
				name: Some(PROFILE_PICTURE_CONSUMER_NAME.into()),
				durable_name: Some(PROFILE_PICTURE_CONSUMER_NAME.into()),
				filter_subject: config.profile_picture_callback_subject.clone(),
				..Default::default()
			},
		)
		.await
		.context("failed to create profile picture consumer")?;

	let mut profile_picture_consumer = profile_picture_consumer
		.messages()
		.await
		.context("failed to get profile picture consumer messages")?;

	loop {
		select! {
			_ = global.ctx().done() => break,
			message = profile_picture_consumer.next() => {
				let message = message.ok_or_else(|| anyhow::anyhow!("profile picture consumer closed"))?.context("failed to get profile picture consumer message")?;
				let job_result = match ProcessedImage::decode(message.payload.as_ref()) {
					Ok(job_result) => job_result,
					Err(err) => {
						tracing::warn!(error = %err, "malformed profile picture job result");
						message.ack().await.map_err(|err| anyhow::anyhow!(err)).context("failed to ack")?;
						continue;
					}
				};
				tracing::debug!("received profile picture job result: {:?}", job_result);

				let mut tx = global.db().begin().await.context("failed to begin transaction")?;

				let uploaded_file: UploadedFile = match sqlx::query_as("UPDATE uploaded_files SET pending = FALSE, metadata = $1, updated_at = NOW() WHERE id = $2 AND pending = TRUE RETURNING *")
					.bind(common::database::Protobuf(UploadedFileMetadata {
						metadata: Some(uploaded_file_metadata::Metadata::Image(uploaded_file_metadata::Image {
							versions: job_result.variants,
						})),
					}))
					.bind(common::database::Ulid(job_result.job_id.into_ulid()))
					.fetch_optional(tx.as_mut())
					.await
					.context("failed to get uploaded file")? {
					Some(uploaded_file) => uploaded_file,
					None => {
						tracing::warn!("uploaded file not found");
						message.ack().await.map_err(|err| anyhow::anyhow!(err)).context("failed to ack")?;
						continue;
					}
				};

				let user_updated = sqlx::query("UPDATE users SET profile_picture_id = $1, pending_profile_picture_id = NULL, updated_at = NOW() WHERE id = $2 AND pending_profile_picture_id = $1")
					.bind(uploaded_file.id)
					.bind(uploaded_file.owner_id)
					.execute(tx.as_mut())
					.await
					.context("failed to update user")?.rows_affected() == 1;

				if let Err(err) = tx.commit().await.context("failed to commit transaction") {
					tracing::warn!(error = %err, "failed to commit transaction");
					message.ack_with(AckKind::Nak(Some(Duration::from_secs(5)))).await.map_err(|err| anyhow::anyhow!(err)).context("failed to ack")?;
					continue;
				}

				message.ack().await.map_err(|err| anyhow::anyhow!(err)).context("failed to ack")?;

				if user_updated {
					global
						.nats()
						.publish(
							SubscriptionTopic::UserProfilePicture(uploaded_file.owner_id.0),
							pb::scuffle::platform::internal::events::UserProfilePicture {
								user_id: Some(uploaded_file.owner_id.0.into()),
								profile_picture_id: Some(uploaded_file.id.0.into()),
							}.encode_to_vec().into(),
						)
						.await
						.context("failed to publish profile picture update")?;
				}
			},
		}
	}

	Ok(())
}
