use pb::scuffle::video::v1::types::access_token_scope::Permission;
use pb::scuffle::video::v1::types::Resource;
use pb::scuffle::video::v1::{RecordingConfigTagRequest, RecordingConfigTagResponse};

use crate::api::utils::impl_request_scopes;
use crate::api::utils::tags::impl_tag_req;
use crate::ratelimit::RateLimitResource;

impl_request_scopes!(
	RecordingConfigTagRequest,
	video_common::database::RecordingConfig,
	(Resource::RecordingConfig, Permission::Modify),
	RateLimitResource::RecordingConfigTag
);

impl_tag_req!(RecordingConfigTagRequest, RecordingConfigTagResponse);
