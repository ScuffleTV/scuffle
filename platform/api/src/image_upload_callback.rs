use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use async_nats::jetstream::stream::RetentionPolicy;
use async_nats::jetstream::AckKind;
use futures_util::StreamExt;
use pb::ext::UlidExt;
use pb::scuffle::platform::internal::events::{processed_image, ProcessedImage};
use pb::scuffle::platform::internal::types::{uploaded_file_metadata, ProcessedImageVariant, UploadedFileMetadata};
use prost::Message;
use utils::context::ContextExt;

use crate::config::ImageUploaderConfig;
use crate::database::{FileType, UploadedFile};
use crate::global::ApiGlobal;
use crate::subscription::SubscriptionTopic;

const CONSUMER_NAME: &str = "image-upload-consumer";

pub async fn run(global: Arc<impl ApiGlobal>) -> anyhow::Result<()> {
	let config = global.config::<ImageUploaderConfig>();

	let image_upload_callback = global
		.jetstream()
		.get_or_create_stream(async_nats::jetstream::stream::Config {
			name: config.callback_subject.clone(),
			subjects: vec![config.callback_subject.clone()],
			max_consumers: 1,
			retention: RetentionPolicy::WorkQueue,
			..Default::default()
		})
		.await
		.context("failed to create profile picture stream")?;

	let image_upload_callback = image_upload_callback
		.get_or_create_consumer(
			CONSUMER_NAME,
			async_nats::jetstream::consumer::pull::Config {
				name: Some(CONSUMER_NAME.to_owned()),
				ack_wait: Duration::from_secs(30),
				..Default::default()
			},
		)
		.await
		.context("failed to create profile picture consumer")?;

	let mut image_upload_consumer = image_upload_callback
		.messages()
		.await
		.context("failed to get profile picture consumer messages")?;

	while let Ok(message) = image_upload_consumer.next().context(global.ctx()).await {
		let message = message
			.ok_or_else(|| anyhow::anyhow!("profile picture consumer closed"))?
			.context("failed to get profile picture consumer message")?;
		let (job_id, job_result) = match ProcessedImage::decode(message.payload.as_ref()) {
			Ok(ProcessedImage {
				job_id,
				result: Some(result),
			}) => (job_id, result),
			err => {
				if let Err(err) = err {
					tracing::warn!(error = %err, "failed to decode profile picture job result");
				} else {
					tracing::warn!("malformed profile picture job result");
				}
				message
					.ack()
					.await
					.map_err(|err| anyhow::anyhow!(err))
					.context("failed to ack")?;
				continue;
			}
		};
		tracing::debug!("received profile picture job result: {:?}", job_result);

		match job_result {
			processed_image::Result::Success(processed_image::Success { variants }) => {
				if let Err(err) = handle_success(&global, job_id.into_ulid(), variants).await {
					tracing::warn!(error = %err, "failed to handle profile picture job success");
					message
						.ack_with(AckKind::Nak(Some(Duration::from_secs(5))))
						.await
						.map_err(|err| anyhow::anyhow!(err))
						.context("failed to ack")?;
				} else {
					message
						.ack()
						.await
						.map_err(|err| anyhow::anyhow!(err))
						.context("failed to ack")?;
				}
			}
			processed_image::Result::Failure(processed_image::Failure {
				reason,
				friendly_message,
			}) => {
				if let Err(err) = handle_failure(&global, job_id.into_ulid(), reason, friendly_message).await {
					tracing::warn!(error = %err, "failed to handle profile picture job failure");
					message
						.ack_with(AckKind::Nak(Some(Duration::from_secs(5))))
						.await
						.map_err(|err| anyhow::anyhow!(err))
						.context("failed to ack")?;
				} else {
					message
						.ack()
						.await
						.map_err(|err| anyhow::anyhow!(err))
						.context("failed to ack")?;
				}
			}
		}

		message
			.ack()
			.await
			.map_err(|err| anyhow::anyhow!(err))
			.context("failed to ack")?;
	}

	Ok(())
}

async fn handle_success(
	global: &Arc<impl ApiGlobal>,
	job_id: ulid::Ulid,
	variants: Vec<ProcessedImageVariant>,
) -> anyhow::Result<()> {
	let mut client = global.db().get().await.context("failed to get db connection")?;
	let tx = client.transaction().await.context("failed to start transaction")?;

	let uploaded_file: UploadedFile = match utils::database::query("UPDATE uploaded_files SET status = 'completed', metadata = $1, updated_at = NOW() WHERE id = $2 AND status = 'queued' RETURNING *")
		.bind(utils::database::Protobuf(UploadedFileMetadata {
			metadata: Some(uploaded_file_metadata::Metadata::Image(uploaded_file_metadata::Image {
				versions: variants,
			})),
		}))
		.bind(job_id)
		.build_query_as()
		.fetch_optional(&tx)
		.await
		.context("failed to get uploaded file")? {
		Some(uploaded_file) => uploaded_file,
		None => {
			anyhow::bail!("uploaded file not found");
		}
	};

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

	match uploaded_file.ty {
		FileType::CategoryArtwork | FileType::CategoryCover => {}
		FileType::ProfilePicture => {
			let user_updated = utils::database::query("UPDATE users SET profile_picture_id = $1, pending_profile_picture_id = NULL, updated_at = NOW() WHERE id = $2 AND pending_profile_picture_id = $1")
				.bind(uploaded_file.id)
				.bind(uploaded_file.owner_id)
				.build()
				.execute(&tx)
				.await
				.context("failed to update user")? == 1;

			tx.commit().await.context("failed to commit transaction")?;

			let owner_id = uploaded_file
				.owner_id
				.ok_or_else(|| anyhow::anyhow!("uploaded file owner id is null"))?;

			if user_updated {
				global
					.nats()
					.publish(
						SubscriptionTopic::UserProfilePicture(owner_id),
						pb::scuffle::platform::internal::events::UserProfilePicture {
							user_id: Some(owner_id.into()),
							profile_picture_id: Some(uploaded_file.id.into()),
						}
						.encode_to_vec()
						.into(),
					)
					.await
					.context("failed to publish profile picture update event")?;
			}
		}
	}

	Ok(())
}

async fn handle_failure(
	global: &Arc<impl ApiGlobal>,
	job_id: ulid::Ulid,
	reason: String,
	friendly_message: String,
) -> anyhow::Result<()> {
	let mut client = global.db().get().await.context("failed to get db connection")?;
	let tx = client.transaction().await.context("failed to start transaction")?;

	let uploaded_file: UploadedFile = match utils::database::query("UPDATE uploaded_files SET status = 'failed', failed = $1, updated_at = NOW() WHERE id = $2 AND status = 'queued' RETURNING *")
		.bind(reason.clone())
		.bind(job_id)
		.build_query_as()
		.fetch_optional(&tx)
		.await
		.context("failed to get uploaded file")? {
		Some(uploaded_file) => uploaded_file,
		None => {
			anyhow::bail!("uploaded file not found");
		}
	};

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

	let update_count = match uploaded_file.ty {
		FileType::CategoryArtwork | FileType::CategoryCover => false,
		FileType::ProfilePicture => {
			utils::database::query(
				"UPDATE users SET pending_profile_picture_id = NULL, updated_at = NOW() WHERE id = $1 AND pending_profile_picture_id = $2",
			)
			.bind(uploaded_file.owner_id)
			.bind(uploaded_file.id)
			.build()
			.execute(&tx)
			.await
			.context("failed to update user")?
				== 1
		}
	};

	tx.commit().await.context("failed to commit transaction")?;

	match (uploaded_file.ty, update_count) {
		(FileType::CategoryArtwork | FileType::CategoryCover, _) => {}
		(FileType::ProfilePicture, true) => {
			global
				.nats()
				.publish(
					SubscriptionTopic::UserProfilePicture(uploaded_file.owner_id.unwrap()),
					pb::scuffle::platform::internal::events::UserProfilePicture {
						user_id: Some(uploaded_file.owner_id.unwrap().into()),
						profile_picture_id: None,
					}
					.encode_to_vec()
					.into(),
				)
				.await
				.context("failed to publish profile picture update event")?;
		}
		(FileType::ProfilePicture, false) => {}
	}

	Ok(())
}
