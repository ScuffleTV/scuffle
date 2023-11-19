use pb::scuffle::video::v1::types::access_token_scope::Permission;
use pb::scuffle::video::v1::types::Resource;
use pb::scuffle::video::v1::{PlaybackKeyPairUntagRequest, PlaybackKeyPairUntagResponse};

use crate::api::utils::impl_request_scopes;
use crate::api::utils::tags::impl_untag_req;
use crate::ratelimit::RateLimitResource;

impl_request_scopes!(
	PlaybackKeyPairUntagRequest,
	video_common::database::PlaybackKeyPair,
	(Resource::PlaybackKeyPair, Permission::Modify),
	RateLimitResource::PlaybackKeyPairUntag
);

impl_untag_req!(PlaybackKeyPairUntagRequest, PlaybackKeyPairUntagResponse);
