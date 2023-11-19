use std::sync::Arc;

use pb::scuffle::video::v1::types::access_token_scope::Permission;
use pb::scuffle::video::v1::types::Resource;
use pb::scuffle::video::v1::{S3BucketCreateRequest, S3BucketCreateResponse};
use tonic::Status;
use ulid::Ulid;
use video_common::database::{AccessToken, DatabaseTable};

use crate::api::utils::tags::validate_tags;
use crate::api::utils::{impl_request_scopes, QbRequest, QbResponse, TonicRequest};
use crate::global::ApiGlobal;
use crate::ratelimit::RateLimitResource;

impl_request_scopes!(
	S3BucketCreateRequest,
	video_common::database::S3Bucket,
	(Resource::S3Bucket, Permission::Create),
	RateLimitResource::S3BucketCreate
);

#[async_trait::async_trait]
impl QbRequest for S3BucketCreateRequest {
	type QueryObject = Self::Table;

	async fn build_query<G: ApiGlobal>(
		&self,
		_: &Arc<G>,
		access_token: &AccessToken,
	) -> tonic::Result<sqlx::QueryBuilder<'_, sqlx::Postgres>> {
		validate_tags(self.tags.as_ref())?;

		let mut qb = sqlx::query_builder::QueryBuilder::default();

		qb.push("INSERT INTO ").push(Self::Table::NAME).push(" (");

		let mut seperated = qb.separated(",");

		seperated.push("id");
		seperated.push("organization_id");
		seperated.push("name");
		seperated.push("region");
		seperated.push("endpoint");
		seperated.push("access_key_id");
		seperated.push("secret_access_key");
		seperated.push("public_url");
		seperated.push("tags");

		qb.push(") VALUES (");

		let mut seperated = qb.separated(",");

		// TODO: check if this is actually secure. How do we prevent SSRF?
		// How do we make sure that these urls point outside of our network?
		if let Some(endpoint) = &self.endpoint {
			url::Url::parse(endpoint).map_err(|_| Status::invalid_argument("invalid endpoint"))?;
		}

		if let Some(public_url) = &self.public_url {
			url::Url::parse(public_url).map_err(|_| Status::invalid_argument("invalid public url"))?;
		}

		seperated.push_bind(common::database::Ulid(Ulid::new()));
		seperated.push_bind(access_token.organization_id);
		seperated.push_bind(&self.name);
		seperated.push_bind(&self.region);
		seperated.push_bind(&self.endpoint);
		seperated.push_bind(&self.access_key_id);
		seperated.push_bind(&self.secret_access_key);
		seperated.push_bind(&self.public_url);
		seperated.push_bind(sqlx::types::Json(self.tags.clone().unwrap_or_default().tags));

		qb.push(") RETURNING *");

		Ok(qb)
	}
}

impl QbResponse for S3BucketCreateResponse {
	type Request = S3BucketCreateRequest;

	fn from_query_object(query_object: Vec<<Self::Request as QbRequest>::QueryObject>) -> tonic::Result<Self> {
		if query_object.is_empty() {
			return Err(Status::internal(format!(
				"failed to create {}, no rows returned",
				<Self::Request as TonicRequest>::Table::FRIENDLY_NAME
			)));
		}

		Ok(Self {
			s3_bucket: Some(query_object.into_iter().next().unwrap().into_proto()),
		})
	}
}
