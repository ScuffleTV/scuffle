use pb::scuffle::video::v1::types::access_token_scope::Permission;
use pb::scuffle::video::v1::types::Resource;
use pb::scuffle::video::v1::{TranscodingConfigTagRequest, TranscodingConfigTagResponse};

use crate::api::utils::impl_request_scopes;
use crate::api::utils::tags::impl_tag_req;
use crate::ratelimit::RateLimitResource;

impl_request_scopes!(
	TranscodingConfigTagRequest,
	video_common::database::TranscodingConfig,
	(Resource::TranscodingConfig, Permission::Modify),
	RateLimitResource::TranscodingConfigTag
);

impl_tag_req!(TranscodingConfigTagRequest, TranscodingConfigTagResponse);
