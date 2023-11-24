use std::sync::Arc;

use pb::scuffle::video::v1::types::access_token_scope::Permission;
use pb::scuffle::video::v1::types::Resource;
use pb::scuffle::video::v1::{AccessTokenCreateRequest, AccessTokenCreateResponse};
use tonic::Status;
use ulid::Ulid;
use video_common::database::{AccessToken, DatabaseTable};

use crate::api::utils::tags::validate_tags;
use crate::api::utils::{impl_request_scopes, AccessTokenExt, QbRequest, QbResponse, RequiredScope, TonicRequest};
use crate::global::ApiGlobal;
use crate::ratelimit::RateLimitResource;

impl_request_scopes!(
	AccessTokenCreateRequest,
	video_common::database::AccessToken,
	(Resource::AccessToken, Permission::Create),
	RateLimitResource::AccessTokenCreate
);

#[async_trait::async_trait]
impl QbRequest for AccessTokenCreateRequest {
	type QueryObject = Self::Table;

	async fn build_query<G: ApiGlobal>(
		&self,
		_: &Arc<G>,
		access_token: &AccessToken,
	) -> tonic::Result<sqlx::QueryBuilder<'_, sqlx::Postgres>> {
		if self.scopes.len() > 20 {
			return Err(Status::invalid_argument("scopes must not be longer than 20"));
		}

		if self.scopes.iter().any(|s| s.permission.len() > 20) {
			return Err(Status::invalid_argument("permission must not be longer than 20"));
		}

		let permissions = RequiredScope(self.scopes.clone()).optimize();

		if self.scopes.is_empty() {
			return Err(Status::invalid_argument("scopes must not be empty"));
		}

		access_token.has_scope(&permissions)?;

		validate_tags(self.tags.as_ref())?;

		let mut qb = sqlx::query_builder::QueryBuilder::default();

		qb.push("INSERT INTO ").push(Self::Table::NAME).push(" (");

		let mut seperated = qb.separated(",");

		seperated.push("id");
		seperated.push("organization_id");
		seperated.push("secret_token");
		seperated.push("scopes");
		seperated.push("last_active_at");
		seperated.push("updated_at");
		seperated.push("expires_at");
		seperated.push("tags");

		qb.push(") VALUES (");

		let mut seperated = qb.separated(",");

		seperated.push_bind(common::database::Ulid(Ulid::new()));
		seperated.push_bind(access_token.organization_id);
		seperated.push_bind(common::database::Ulid(Ulid::new()));
		seperated.push_bind(permissions.0.into_iter().map(common::database::Protobuf).collect::<Vec<_>>());
		seperated.push_bind(None::<chrono::DateTime<chrono::Utc>>);
		seperated.push_bind(chrono::Utc::now());
		seperated.push_bind(self.expires_at);
		seperated.push_bind(sqlx::types::Json(self.tags.clone().unwrap_or_default().tags));

		qb.push(") RETURNING *");

		Ok(qb)
	}
}

impl QbResponse for AccessTokenCreateResponse {
	type Request = AccessTokenCreateRequest;

	fn from_query_object(query_object: Vec<<Self::Request as QbRequest>::QueryObject>) -> tonic::Result<Self> {
		if query_object.len() != 1 {
			return Err(Status::internal(format!(
				"failed to create {}, {} rows returned",
				<Self::Request as TonicRequest>::Table::FRIENDLY_NAME,
				query_object.len(),
			)));
		}

		let query_object = query_object.into_iter().next().unwrap();

		Ok(Self {
			secret: query_object.secret_token.to_string(),
			access_token: Some(query_object.into_proto()),
		})
	}
}
