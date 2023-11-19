use pb::scuffle::video::v1::types::access_token_scope::Permission;
use pb::scuffle::video::v1::types::Resource;
use pb::scuffle::video::v1::{AccessTokenUntagRequest, AccessTokenUntagResponse};

use crate::api::utils::impl_request_scopes;
use crate::api::utils::tags::impl_untag_req;
use crate::ratelimit::RateLimitResource;

impl_request_scopes!(
	AccessTokenUntagRequest,
	video_common::database::AccessToken,
	(Resource::AccessToken, Permission::Modify),
	RateLimitResource::AccessTokenUntag
);

impl_untag_req!(AccessTokenUntagRequest, AccessTokenUntagResponse);
