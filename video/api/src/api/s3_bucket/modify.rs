use std::sync::Arc;

use pb::ext::UlidExt;
use pb::scuffle::video::v1::types::access_token_scope::Permission;
use pb::scuffle::video::v1::types::Resource;
use pb::scuffle::video::v1::{S3BucketModifyRequest, S3BucketModifyResponse};
use video_common::database::{AccessToken, DatabaseTable};

use super::utils::{
	validate_access_key_id, validate_endpoint, validate_name, validate_public_url, validate_region,
	validate_secret_access_key,
};
use crate::api::errors::MODIFY_NO_FIELDS;
use crate::api::utils::tags::validate_tags;
use crate::api::utils::{impl_request_scopes, QbRequest, QbResponse, TonicRequest};
use crate::global::ApiGlobal;
use crate::ratelimit::RateLimitResource;

impl_request_scopes!(
	S3BucketModifyRequest,
	video_common::database::S3Bucket,
	(Resource::S3Bucket, Permission::Modify),
	RateLimitResource::S3BucketModify
);

#[async_trait::async_trait]
impl QbRequest for S3BucketModifyRequest {
	type QueryObject = Self::Table;

	async fn build_query<G: ApiGlobal>(
		&self,
		_: &Arc<G>,
		access_token: &AccessToken,
	) -> tonic::Result<sqlx::QueryBuilder<'_, sqlx::Postgres>> {
		validate_tags(self.tags.as_ref())?;

		let mut qb = sqlx::query_builder::QueryBuilder::default();

		qb.push("UPDATE ").push(Self::Table::NAME).push(" SET ");

		let mut seperated = qb.separated(",");

		if let Some(access_key_id) = &self.access_key_id {
			validate_access_key_id(access_key_id)?;
			seperated.push("access_key_id = ").push_bind_unseparated(access_key_id);
		}

		if let Some(secret_access_key) = &self.secret_access_key {
			validate_secret_access_key(secret_access_key)?;
			seperated
				.push("secret_access_key = ")
				.push_bind_unseparated(secret_access_key);
		}

		if let Some(name) = &self.name {
			validate_name(name)?;
			seperated.push("name = ").push_bind_unseparated(name);
		}

		if let Some(region) = &self.region {
			validate_region(region)?;
			seperated.push("region = ").push_bind_unseparated(region);
		}

		if let Some(endpoint) = &self.endpoint {
			if endpoint.is_empty() {
				seperated.push("endpoint = NULL");
			} else {
				validate_endpoint(endpoint)?;
				seperated.push("endpoint = ").push_bind_unseparated(endpoint);
			}
		}

		if let Some(public_url) = &self.public_url {
			if public_url.is_empty() {
				seperated.push("public_url = NULL");
			} else {
				validate_public_url(public_url)?;
				seperated.push("public_url = ").push_bind_unseparated(public_url);
			}
		}

		if let Some(tags) = &self.tags {
			seperated.push("tags = ").push_bind_unseparated(sqlx::types::Json(&tags.tags));
		}

		if self.tags.is_none()
			&& self.access_key_id.is_none()
			&& self.secret_access_key.is_none()
			&& self.name.is_none()
			&& self.region.is_none()
			&& self.endpoint.is_none()
			&& self.public_url.is_none()
		{
			return Err(tonic::Status::invalid_argument(MODIFY_NO_FIELDS));
		}

		seperated.push("updated_at = NOW()");

		qb.push(" WHERE id = ").push_bind(common::database::Ulid(self.id.into_ulid()));
		qb.push(" AND organization_id = ").push_bind(access_token.organization_id);
		qb.push(" RETURNING *");

		Ok(qb)
	}
}

impl QbResponse for S3BucketModifyResponse {
	type Request = S3BucketModifyRequest;

	fn from_query_object(query_object: Vec<<Self::Request as QbRequest>::QueryObject>) -> tonic::Result<Self> {
		if query_object.is_empty() {
			return Err(tonic::Status::not_found(format!(
				"{} not found",
				<Self::Request as TonicRequest>::Table::FRIENDLY_NAME
			)));
		}

		if query_object.len() > 1 {
			return Err(tonic::Status::internal(format!(
				"failed to modify {}, {} rows returned",
				<Self::Request as TonicRequest>::Table::FRIENDLY_NAME,
				query_object.len(),
			)));
		}

		Ok(Self {
			s3_bucket: Some(query_object.into_iter().next().unwrap().into_proto()),
		})
	}
}
