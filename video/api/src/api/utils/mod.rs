pub mod access_tokens;
pub mod auth;
pub mod get;
pub mod ratelimit;
pub mod tags;

use std::sync::Arc;

pub use access_tokens::{AccessTokenExt, RequiredScope};
pub use ratelimit::TonicRequest;
use tonic::Status;
use video_common::database::{AccessToken, DatabaseTable};

use crate::global::ApiGlobal;

macro_rules! impl_request_scopes {
	($type:ty, $table:ty, $permission:expr, $ratelimit:expr) => {
		impl crate::api::utils::TonicRequest for $type {
			type Table = $table;

			#[inline(always)]
			fn permission_scope(&self) -> crate::api::utils::RequiredScope {
				($permission).into()
			}

			#[inline(always)]
			fn ratelimit_scope(&self) -> crate::ratelimit::RateLimitResource {
				($ratelimit).into()
			}
		}
	};
}

pub(crate) use impl_request_scopes;

#[async_trait::async_trait]
pub trait ApiRequest<R> {
	async fn process<G: ApiGlobal>(&self, global: &Arc<G>, access_token: &AccessToken) -> tonic::Result<tonic::Response<R>>;
}

#[async_trait::async_trait]
pub trait QbRequest: TonicRequest {
	type QueryObject;

	async fn build_query<G: ApiGlobal>(
		&self,
		global: &Arc<G>,
		access_token: &AccessToken,
	) -> tonic::Result<sqlx::QueryBuilder<'_, sqlx::Postgres>>;
}

pub trait QbResponse: Sized {
	type Request: QbRequest;

	fn from_query_object(query_object: Vec<<Self::Request as QbRequest>::QueryObject>) -> tonic::Result<Self>;
}

#[async_trait::async_trait]
impl<T: QbRequest, R> ApiRequest<R> for tonic::Request<T>
where
	for<'r> T::QueryObject: sqlx::FromRow<'r, sqlx::postgres::PgRow> + Send + Sync + Unpin,
	T::Table: DatabaseTable,
	R: QbResponse<Request = T>,
	Self: Send + Sync,
{
	async fn process<G: ApiGlobal>(&self, global: &Arc<G>, access_token: &AccessToken) -> tonic::Result<tonic::Response<R>> {
		let mut query_builder = self.get_ref().build_query(global, access_token).await?;

		let results: Vec<T::QueryObject> =
			query_builder
				.build_query_as()
				.fetch_all(global.db().as_ref())
				.await
				.map_err(|err| {
					tracing::error!(err = %err, "failed to fetch {}s", <T::Table as DatabaseTable>::FRIENDLY_NAME);
					Status::internal(format!("failed to fetch {}s", <T::Table as DatabaseTable>::FRIENDLY_NAME))
				})?;

		Ok(tonic::Response::new(R::from_query_object(results)?))
	}
}
