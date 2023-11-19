use pb::scuffle::video::v1::types::access_token_scope::Permission;
use pb::scuffle::video::v1::types::Resource;
use pb::scuffle::video::v1::{AccessTokenTagRequest, AccessTokenTagResponse};

use crate::api::utils::impl_request_scopes;
use crate::api::utils::tags::impl_tag_req;
use crate::ratelimit::RateLimitResource;

impl_request_scopes!(
	AccessTokenTagRequest,
	video_common::database::AccessToken,
	(Resource::AccessToken, Permission::Modify),
	RateLimitResource::AccessTokenTag
);

impl_tag_req!(AccessTokenTagRequest, AccessTokenTagResponse);
