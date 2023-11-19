use pb::scuffle::video::v1::types::access_token_scope::Permission;
use pb::scuffle::video::v1::types::Resource;
use pb::scuffle::video::v1::{S3BucketTagRequest, S3BucketTagResponse};

use crate::api::utils::impl_request_scopes;
use crate::api::utils::tags::impl_tag_req;
use crate::ratelimit::RateLimitResource;

impl_request_scopes!(
	S3BucketTagRequest,
	video_common::database::S3Bucket,
	(Resource::S3Bucket, Permission::Modify),
	RateLimitResource::S3BucketTag
);

impl_tag_req!(S3BucketTagRequest, S3BucketTagResponse);
