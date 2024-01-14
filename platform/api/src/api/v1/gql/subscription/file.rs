use async_graphql::{Context, Enum, SimpleObject, Subscription};
use futures_util::Stream;
use pb::ext::*;
use prost::Message;

use crate::api::v1::gql::error::ext::{OptionExt, ResultExt};
use crate::api::v1::gql::error::{GqlError, Result};
use crate::api::v1::gql::ext::ContextExt;
use crate::subscription::SubscriptionTopic;
use crate::{api::v1::gql::models::ulid::GqlUlid, global::ApiGlobal};

pub struct FileSubscription<G: ApiGlobal>(std::marker::PhantomData<G>);

impl<G: ApiGlobal> Default for FileSubscription<G> {
	fn default() -> Self {
		Self(std::marker::PhantomData)
	}
}

#[derive(SimpleObject)]
struct FileStatusStream {
	/// The ID of the file.
	pub file_id: GqlUlid,
	/// The status of the file.
	pub status: FileStatus,
	/// Only set if status is `Failure`.
	pub reason: Option<String>,
	/// Only set if status is `Failure`.
	pub friendly_message: Option<String>,
}

#[derive(Enum, Copy, Clone, Eq, PartialEq)]
enum FileStatus {
	Success,
	Failure,
}

#[Subscription]
impl<G: ApiGlobal> FileSubscription<G> {
	async fn file_status<'ctx>(
		&self,
		ctx: &'ctx Context<'ctx>,
		file_id: GqlUlid,
	) -> Result<impl Stream<Item = Result<FileStatusStream>> + 'ctx> {
		let global = ctx.get_global::<G>();

		// TODO: get initial status
		let file = global
			.uploaded_file_by_id_loader()
			.load(file_id.to_ulid())
			.await
			.map_err_ignored_gql("failed to load file")?
			.map_err_gql(GqlError::InvalidInput {
				fields: vec!["fileId"],
				message: "file not found",
			})?;

		let mut subscription = global
			.subscription_manager()
			.subscribe(SubscriptionTopic::UploadedFileStatus(file_id.to_ulid()))
			.await
			.map_err_gql("failed to subscribe to file status")?;

		Ok(async_stream::stream!({
			if !file.pending {
				// When file isn't pending anymore, just yield once with the status from the db
				let status = if file.failed.is_some() {
					FileStatus::Failure
				} else {
					FileStatus::Success
				};
				yield Ok(FileStatusStream {
					file_id: file.id.0.into(),
					status,
					reason: file.failed,
					// TODO: we don't have access to the friendly message here because it isn't in the db
					friendly_message: None,
				});
			} else {
				// Only receive one message
				if let Ok(message) = subscription.recv().await {
					let event = pb::scuffle::platform::internal::events::UploadedFileStatus::decode(message.payload)
						.map_err_ignored_gql("failed to decode uploaded file status event")?;

					let file_id = event.file_id.into_ulid();
					let (status, reason, friendly_message) = match event.status.unwrap() {
						pb::scuffle::platform::internal::events::uploaded_file_status::Status::Success(_) => {
							(FileStatus::Success, None, None)
						}
						pb::scuffle::platform::internal::events::uploaded_file_status::Status::Failure(
							pb::scuffle::platform::internal::events::uploaded_file_status::Failure {
								reason,
								friendly_message,
							},
						) => (FileStatus::Failure, Some(reason), Some(friendly_message)),
					};

					yield Ok(FileStatusStream {
						file_id: file_id.into(),
						status,
						reason,
						friendly_message,
					});
				}
			}
		}))
	}
}
