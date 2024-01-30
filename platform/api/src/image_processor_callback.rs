use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use async_nats::jetstream::consumer::pull::MessagesErrorKind;
use async_nats::jetstream::stream::RetentionPolicy;
use async_nats::jetstream::AckKind;
use futures_util::StreamExt;
use pb::ext::UlidExt;
use pb::scuffle::platform::internal::events::{processed_image, ProcessedImage};
use pb::scuffle::platform::internal::types::{uploaded_file_metadata, UploadedFileMetadata};
use prost::Message;
use utils::context::ContextExt;

use crate::config::ImageUploaderConfig;
use crate::database::{FileType, UploadedFile, UploadedFileStatus};
use crate::global::ApiGlobal;
use crate::subscription::SubscriptionTopic;

pub async fn run<G: ApiGlobal>(global: Arc<G>) -> anyhow::Result<()> {
	let config = global.config::<ImageUploaderConfig>();

	// It can't contain dots for some reason
	let stream_name = config.callback_subject.replace('.', "-");

	let stream = global
		.jetstream()
		.get_or_create_stream(async_nats::jetstream::stream::Config {
			name: stream_name.clone(),
			subjects: vec![config.callback_subject.clone()],
			max_consumers: 1,
			retention: RetentionPolicy::WorkQueue,
			..Default::default()
		})
		.await
		.context("stream")?;

	let consumer = stream
		.get_or_create_consumer(
			&stream_name,
			async_nats::jetstream::consumer::pull::Config {
				name: Some(stream_name.clone()),
				..Default::default()
			},
		)
		.await
		.context("consumer")?;

	let mut messages = consumer.messages().await.context("messages")?;

	while let Ok(message) = messages.next().context(global.ctx()).await {
		handle_message(&global, message).await?;
	}

	Ok(())
}

async fn handle_message<G: ApiGlobal>(
	global: &Arc<G>,
	message: Option<Result<async_nats::jetstream::Message, async_nats::error::Error<MessagesErrorKind>>>,
) -> anyhow::Result<()> {
	let message = match message {
		Some(Ok(message)) => message,
		Some(Err(err)) if matches!(err.kind(), MessagesErrorKind::MissingHeartbeat) => {
			tracing::warn!("missing heartbeat");
			return Ok(());
		}
		Some(Err(err)) => {
			anyhow::bail!("message: {:#}", err)
		}
		None => {
			anyhow::bail!("stream closed");
		}
	};

	let (job_id, job_result) = match ProcessedImage::decode(message.payload.as_ref()) {
		Ok(ProcessedImage {
			job_id,
			result: Some(result),
		}) => (job_id, result),
		err => {
			if let Err(err) = err {
				tracing::warn!(error = %err, "failed to decode image upload job result");
			} else {
				tracing::warn!("malformed image upload job result");
			}
			message
				.ack()
				.await
				.map_err(|err| anyhow::anyhow!(err))
				.context("failed to ack")?;
			return Ok(());
		}
	};
	tracing::trace!("received image upload job result: {:?}", job_result);

	let mut client = global.db().get().await.context("failed to get db connection")?;
	let tx = client.transaction().await.context("failed to start transaction")?;

	let uploaded_file: UploadedFile = match utils::database::query("UPDATE uploaded_files SET status = $1, failed = $2, metadata = $3, updated_at = NOW() WHERE id = $4 AND status = 'queued' RETURNING *")
		.bind(if matches!(job_result, processed_image::Result::Success(_)) {
			UploadedFileStatus::Completed
		} else {
			UploadedFileStatus::Failed
		})
		.bind(match &job_result {
			processed_image::Result::Success(_) => None,
			processed_image::Result::Failure(processed_image::Failure { reason, .. }) => {
				Some(reason)
			}
		})
		.bind(utils::database::Protobuf(UploadedFileMetadata {
			metadata: Some(uploaded_file_metadata::Metadata::Image(uploaded_file_metadata::Image {
				versions: match &job_result {
					processed_image::Result::Success(processed_image::Success { variants }) => variants.clone(),
					processed_image::Result::Failure(_) => Vec::new(),
				},
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
			return Ok(());
		}
	};

	match job_result {
		processed_image::Result::Success(_) => {
			global
				.nats()
				.publish(
					SubscriptionTopic::UploadedFileStatus(uploaded_file.id),
					pb::scuffle::platform::internal::events::UploadedFileStatus {
						file_id: Some(uploaded_file.id.into()),
						status: Some(
							pb::scuffle::platform::internal::events::uploaded_file_status::Status::Success(
								pb::scuffle::platform::internal::events::uploaded_file_status::Success {},
							),
						),
					}
					.encode_to_vec()
					.into(),
				)
				.await
				.context("failed to publish file update event")?;

			let updated = match uploaded_file.ty {
				FileType::ProfilePicture => {
					let owner_id = uploaded_file
						.owner_id
						.ok_or_else(|| anyhow::anyhow!("uploaded file owner id is null"))?;

					if utils::database::query("UPDATE users SET profile_picture_id = $1, pending_profile_picture_id = NULL, updated_at = NOW() WHERE id = $2 AND pending_profile_picture_id = $3")
						.bind(uploaded_file.id)
						.bind(owner_id)
						.bind(uploaded_file.id)
						.build()
						.execute(&tx)
						.await
						.context("failed to update user")? == 1 {
						Some((
							SubscriptionTopic::UserProfilePicture(uploaded_file.owner_id.unwrap()),
							pb::scuffle::platform::internal::events::UserProfilePicture {
								user_id: Some(uploaded_file.owner_id.unwrap().into()),
								profile_picture_id: Some(uploaded_file.id.into()),
							}
							.encode_to_vec()
							.into(),
						))
					} else {
						None
					}
				}
				FileType::CategoryCover => None,
				FileType::CategoryArtwork => None,
			};

			if let Some((topic, payload)) = updated {
				global
					.nats()
					.publish(topic, payload)
					.await
					.context("failed to publish image upload update event")?;
			}
		}
		processed_image::Result::Failure(processed_image::Failure {
			reason,
			friendly_message,
		}) => {
			global
				.nats()
				.publish(
					SubscriptionTopic::UploadedFileStatus(uploaded_file.id),
					pb::scuffle::platform::internal::events::UploadedFileStatus {
						file_id: Some(uploaded_file.id.into()),
						status: Some(
							pb::scuffle::platform::internal::events::uploaded_file_status::Status::Failure(
								pb::scuffle::platform::internal::events::uploaded_file_status::Failure {
									reason,
									friendly_message,
								},
							),
						),
					}
					.encode_to_vec()
					.into(),
				)
				.await
				.context("failed to publish file update event")?;

			match uploaded_file.ty {
				FileType::ProfilePicture => {
					let owner_id = uploaded_file
						.owner_id
						.ok_or_else(|| anyhow::anyhow!("uploaded file owner id is null"))?;

					utils::database::query(
						"UPDATE users SET pending_profile_picture_id = NULL, updated_at = NOW() WHERE id = $1 AND pending_profile_picture_id = $2",
					)
					.bind(owner_id)
					.bind(uploaded_file.id)
					.build()
					.execute(&tx)
					.await
					.context("failed to update user")?;
				}
				FileType::CategoryCover => {}
				FileType::CategoryArtwork => {}
			}
		}
	}

	if let Err(err) = tx.commit().await.context("failed to commit transaction") {
		tracing::warn!(error = %err, "failed to commit transaction");
		message
			.ack_with(AckKind::Nak(Some(Duration::from_secs(5))))
			.await
			.map_err(|err| anyhow::anyhow!(err))
			.context("failed to ack")?;
		return Ok(());
	}

	message
		.ack()
		.await
		.map_err(|err| anyhow::anyhow!(err))
		.context("failed to ack")?;

	tracing::debug!("processed image upload job result");
	Ok(())
}
