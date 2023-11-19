use pb::scuffle::video::v1::types::access_token_scope::Permission;
use pb::scuffle::video::v1::types::Resource;
use pb::scuffle::video::v1::{PlaybackKeyPairTagRequest, PlaybackKeyPairTagResponse};

use crate::api::utils::impl_request_scopes;
use crate::api::utils::tags::impl_tag_req;
use crate::ratelimit::RateLimitResource;

impl_request_scopes!(
	PlaybackKeyPairTagRequest,
	video_common::database::PlaybackKeyPair,
	(Resource::PlaybackKeyPair, Permission::Modify),
	RateLimitResource::PlaybackKeyPairTag
);

impl_tag_req!(PlaybackKeyPairTagRequest, PlaybackKeyPairTagResponse);
