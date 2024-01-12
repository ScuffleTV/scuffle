use async_graphql::{Context, Enum, SimpleObject, Subscription};
use futures_util::Stream;
use pb::ext::*;
use prost::Message;

use crate::api::v1::gql::error::ext::ResultExt;
use crate::api::v1::gql::error::Result;
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

		let mut subscription = global
			.subscription_manager()
			.subscribe(SubscriptionTopic::UploadedFileStatus(file_id.to_ulid()))
			.await
			.map_err_gql("failed to subscribe to file status")?;

		Ok(async_stream::stream!({
			while let Ok(message) = subscription.recv().await {
				let event = pb::scuffle::platform::internal::events::UploadedFileStatus::decode(message.payload)
					.map_err_ignored_gql("failed to decode uploaded file status event")?;

				let file_id = event.file_id.into_ulid();
				let (status, reason) = match event.status.unwrap() {
					pb::scuffle::platform::internal::events::uploaded_file_status::Status::Success(_) => {
						(FileStatus::Success, None)
					}
					pb::scuffle::platform::internal::events::uploaded_file_status::Status::Failure(
						pb::scuffle::platform::internal::events::uploaded_file_status::Failure { reason },
					) => (FileStatus::Failure, Some(reason)),
				};

				yield Ok(FileStatusStream {
					file_id: file_id.into(),
					status,
					reason,
				});
			}
		}))
	}
}
