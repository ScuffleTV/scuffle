use pb::scuffle::video::v1::types::access_token_scope::Permission;
use pb::scuffle::video::v1::types::Resource;
use pb::scuffle::video::v1::{S3BucketUntagRequest, S3BucketUntagResponse};

use crate::api::utils::impl_request_scopes;
use crate::api::utils::tags::impl_untag_req;
use crate::ratelimit::RateLimitResource;

impl_request_scopes!(
	S3BucketUntagRequest,
	video_common::database::S3Bucket,
	(Resource::S3Bucket, Permission::Modify),
	RateLimitResource::S3BucketUntag
);

impl_untag_req!(S3BucketUntagRequest, S3BucketUntagResponse);
