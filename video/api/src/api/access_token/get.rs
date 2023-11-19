use std::sync::Arc;

use pb::scuffle::video::v1::types::access_token_scope::Permission;
use pb::scuffle::video::v1::types::Resource;
use pb::scuffle::video::v1::{AccessTokenGetRequest, AccessTokenGetResponse};
use video_common::database::{AccessToken, DatabaseTable};

use crate::api::utils::{get, impl_request_scopes, QbRequest, QbResponse};
use crate::global::ApiGlobal;
use crate::ratelimit::RateLimitResource;

impl_request_scopes!(
	AccessTokenGetRequest,
	video_common::database::AccessToken,
	(Resource::AccessToken, Permission::Read),
	RateLimitResource::AccessTokenGet
);

#[async_trait::async_trait]
impl QbRequest for AccessTokenGetRequest {
	type QueryObject = Self::Table;

	async fn build_query<G: ApiGlobal>(
		&self,
		_: &Arc<G>,
		access_token: &AccessToken,
	) -> tonic::Result<sqlx::QueryBuilder<'_, sqlx::Postgres>> {
		let mut qb = sqlx::query_builder::QueryBuilder::default();
		qb.push("SELECT * FROM ").push(Self::Table::NAME).push(" WHERE ");

		let mut seperated = qb.separated(" AND ");

		get::organization_id(&mut seperated, access_token.organization_id);
		get::ids(&mut seperated, &self.ids);
		get::search_options(&mut seperated, self.search_options.as_ref())?;

		Ok(qb)
	}
}

impl QbResponse for AccessTokenGetResponse {
	type Request = AccessTokenGetRequest;

	fn from_query_object(query_objects: Vec<<Self::Request as QbRequest>::QueryObject>) -> tonic::Result<Self> {
		Ok(Self {
			access_tokens: query_objects
				.into_iter()
				.map(video_common::database::AccessToken::into_proto)
				.collect(),
		})
	}
}
