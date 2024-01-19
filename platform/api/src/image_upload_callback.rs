use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use async_nats::jetstream::AckKind;
use futures_util::StreamExt;
use pb::ext::UlidExt;
use pb::scuffle::platform::internal::events::{processed_image, ProcessedImage};
use pb::scuffle::platform::internal::types::{uploaded_file_metadata, UploadedFileMetadata};
use prost::Message;
use tokio::select;

use crate::config::ImageUploaderConfig;
use crate::database::UploadedFile;
use crate::global::ApiGlobal;
use crate::subscription::SubscriptionTopic;

const IMAGE_UPLOAD_CONSUMER_NAME: &str = "image-upload-consumer";

enum Subject {
	ProfilePicture,
	OfflineBanner,
}

pub async fn run<G: ApiGlobal>(global: Arc<G>) -> anyhow::Result<()> {
	let config = global.config::<ImageUploaderConfig>();

	let wildcard = format!("{}.>", config.callback_subject);

	// It can't contain dots for some reason
	let stream_name = config.callback_subject.replace(".", "-");

	let image_upload_stream = global
		.jetstream()
		.get_or_create_stream(async_nats::jetstream::stream::Config {
			name: stream_name,
			subjects: vec![wildcard.clone()],
			max_consumers: 1,
			..Default::default()
		})
		.await
		.context("failed to create image upload stream")?;

	let image_upload_consumer = image_upload_stream
		.get_or_create_consumer(
			IMAGE_UPLOAD_CONSUMER_NAME,
			async_nats::jetstream::consumer::pull::Config {
				name: Some(IMAGE_UPLOAD_CONSUMER_NAME.into()),
				durable_name: Some(IMAGE_UPLOAD_CONSUMER_NAME.into()),
				filter_subject: wildcard.clone(),
				..Default::default()
			},
		)
		.await
		.context("failed to create image upload consumer")?;

	let mut image_upload_consumer = image_upload_consumer
		.messages()
		.await
		.context("failed to get image upload consumer messages")?;

	loop {
		select! {
			_ = global.ctx().done() => break,
			message = image_upload_consumer.next() => {
				let message = message.ok_or_else(|| anyhow::anyhow!("image upload consumer closed"))?.context("failed to get image upload consumer message")?;

				let subject = if message.subject.ends_with(&config.profile_picture_suffix) {
					Subject::ProfilePicture
				} else if message.subject.ends_with(&config.offline_banner_suffix) {
					Subject::OfflineBanner
				} else {
					tracing::warn!("unknown image upload subject: {}", message.subject);
					message.ack().await.map_err(|err| anyhow::anyhow!(err)).context("failed to ack")?;
					continue;
				};

				let (job_id, job_result) = match ProcessedImage::decode(message.payload.as_ref()) {
					Ok(ProcessedImage { job_id, result: Some(result) }) => (job_id, result),
					err => {
						if let Err(err) = err {
							tracing::warn!(error = %err, "failed to decode image upload job result");
						} else {
							tracing::warn!("malformed image upload job result");
						}
						message.ack().await.map_err(|err| anyhow::anyhow!(err)).context("failed to ack")?;
						continue;
					},
				};
				tracing::debug!("received image upload job result: {:?}", job_result);

				let mut client = global.db().get().await.context("failed to get db connection")?;
				let tx = client.transaction().await.context("failed to start transaction")?;

				match job_result {
					processed_image::Result::Success(processed_image::Success { variants }) => {
						let uploaded_file: UploadedFile = match common::database::query("UPDATE uploaded_files SET pending = FALSE, metadata = $1, updated_at = NOW() WHERE id = $2 AND pending = TRUE RETURNING *")
							.bind(common::database::Protobuf(UploadedFileMetadata {
								metadata: Some(uploaded_file_metadata::Metadata::Image(uploaded_file_metadata::Image {
									versions: variants,
								})),
							}))
							.bind(job_id.into_ulid())
							.build_query_as()
							.fetch_optional(&tx)
							.await
							.context("failed to get uploaded file")? {
							Some(uploaded_file) => uploaded_file,
							None => {
								tracing::warn!("uploaded file not found");
								message.ack().await.map_err(|err| anyhow::anyhow!(err)).context("failed to ack")?;
								continue;
							}
						};

						global
							.nats()
							.publish(
								SubscriptionTopic::UploadedFileStatus(uploaded_file.id),
								pb::scuffle::platform::internal::events::UploadedFileStatus {
									file_id: Some(uploaded_file.id.into()),
									status: Some(pb::scuffle::platform::internal::events::uploaded_file_status::Status::Success(pb::scuffle::platform::internal::events::uploaded_file_status::Success {})),
								}.encode_to_vec().into(),
							)
							.await
							.context("failed to publish file update event")?;

						let mut qb = common::database::query("UPDATE users SET ");
						let (pending_column, column) = match subject {
							Subject::ProfilePicture => ("pending_profile_picture_id", "profile_picture_id"),
							Subject::OfflineBanner => ("pending_offline_banner_id", "offline_banner_id"),
						};
						qb.push(column).push(" = ").push_bind(uploaded_file.id).push(", ").push(pending_column).push(" = NULL, updated_at = NOW() WHERE id = ").push_bind(uploaded_file.owner_id).push(" AND ").push(pending_column).push(" = ").push_bind(uploaded_file.id);
						let user_updated = qb.build().execute(&tx).await.context("failed to update user")? == 1;

						if let Err(err) = tx.commit().await.context("failed to commit transaction") {
							tracing::warn!(error = %err, "failed to commit transaction");
							message.ack_with(AckKind::Nak(Some(Duration::from_secs(5)))).await.map_err(|err| anyhow::anyhow!(err)).context("failed to ack")?;
							continue;
						}

						if user_updated {
							let event_subject = match subject {
								Subject::ProfilePicture => SubscriptionTopic::UserProfilePicture(uploaded_file.owner_id),
								Subject::OfflineBanner => SubscriptionTopic::ChannelOfflineBanner(uploaded_file.owner_id),
							};
							let payload = match subject {
								Subject::ProfilePicture => pb::scuffle::platform::internal::events::UserProfilePicture {
									user_id: Some(uploaded_file.owner_id.into()),
									profile_picture_id: Some(uploaded_file.id.into()),
								}.encode_to_vec().into(),
								Subject::OfflineBanner => pb::scuffle::platform::internal::events::ChannelOfflineBanner {
									channel_id: Some(uploaded_file.owner_id.into()),
									offline_banner_id: Some(uploaded_file.id.into()),
								}.encode_to_vec().into(),
							};

							global
								.nats()
								.publish(event_subject, payload)
								.await
								.context("failed to publish image upload update event")?;
						}
					},
					processed_image::Result::Failure(processed_image::Failure { reason, friendly_message }) => {
						let uploaded_file: UploadedFile = match common::database::query("UPDATE uploaded_files SET pending = FALSE, failed = $1, updated_at = NOW() WHERE id = $2 AND pending = TRUE RETURNING *")
							.bind(reason.clone())
							.bind(job_id.into_ulid())
							.build_query_as()
							.fetch_optional(&tx)
							.await
							.context("failed to get uploaded file")? {
							Some(uploaded_file) => uploaded_file,
							None => {
								tracing::warn!("uploaded file not found");
								message.ack().await.map_err(|err| anyhow::anyhow!(err)).context("failed to ack")?;
								continue;
							}
						};

						global
							.nats()
							.publish(
								SubscriptionTopic::UploadedFileStatus(uploaded_file.id),
								pb::scuffle::platform::internal::events::UploadedFileStatus {
									file_id: Some(uploaded_file.id.into()),
									status: Some(pb::scuffle::platform::internal::events::uploaded_file_status::Status::Failure(pb::scuffle::platform::internal::events::uploaded_file_status::Failure {
										reason,
										friendly_message,
									})),
								}.encode_to_vec().into(),
							)
							.await
							.context("failed to publish file update event")?;

						common::database::query("UPDATE users SET pending_profile_picture_id = NULL, updated_at = NOW() WHERE id = $1 AND pending_profile_picture_id = $2")
							.bind(uploaded_file.owner_id)
							.bind(uploaded_file.id)
							.build()
							.execute(&tx)
							.await
							.context("failed to update user")?;

						if let Err(err) = tx.commit().await.context("failed to commit transaction") {
							tracing::warn!(error = %err, "failed to commit transaction");
							message.ack_with(AckKind::Nak(Some(Duration::from_secs(5)))).await.map_err(|err| anyhow::anyhow!(err)).context("failed to ack")?;
							continue;
						}
					},
				}

				message.ack().await.map_err(|err| anyhow::anyhow!(err)).context("failed to ack")?;
			},
		}
	}

	Ok(())
}
