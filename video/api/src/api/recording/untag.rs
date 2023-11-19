use pb::scuffle::video::v1::types::access_token_scope::Permission;
use pb::scuffle::video::v1::types::Resource;
use pb::scuffle::video::v1::{RecordingUntagRequest, RecordingUntagResponse};

use crate::api::utils::impl_request_scopes;
use crate::api::utils::tags::impl_untag_req;
use crate::ratelimit::RateLimitResource;

impl_request_scopes!(
	RecordingUntagRequest,
	video_common::database::Recording,
	(Resource::Recording, Permission::Modify),
	RateLimitResource::RecordingUntag
);

impl_untag_req!(RecordingUntagRequest, RecordingUntagResponse);
