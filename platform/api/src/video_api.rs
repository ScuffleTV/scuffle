use std::time::Duration;

use base64::Engine;
use pb::scuffle::video::v1::events_client::EventsClient;
use pb::scuffle::video::v1::playback_session_client::PlaybackSessionClient;
use pb::scuffle::video::v1::room_client::RoomClient;
use tonic::service::interceptor::InterceptedService;
use tonic::transport::Channel;
use ulid::Ulid;

use crate::config::VideoApiConfig;

#[derive(Clone, Copy)]
pub struct AuthInterceptor {
	organization_id: Ulid,
	access_key: Ulid,
	secret_key: Ulid,
}

impl tonic::service::Interceptor for AuthInterceptor {
	fn call(&mut self, mut request: tonic::Request<()>) -> tonic::Result<tonic::Request<()>> {
		request
			.metadata_mut()
			.insert("x-scuffle-organization-id", self.organization_id.to_string().parse().unwrap());

		let auth =
			base64::engine::general_purpose::URL_SAFE.encode(format!("{}:{}", self.access_key, self.secret_key).as_bytes());

		request
			.metadata_mut()
			.insert("authorization", format!("Basic {auth}").parse().unwrap());
		Ok(request)
	}
}

pub type VideoRoomClient = RoomClient<InterceptedService<Channel, AuthInterceptor>>;
pub type VideoPlaybackSessionClient = PlaybackSessionClient<InterceptedService<Channel, AuthInterceptor>>;
pub type VideoEventsClient = EventsClient<InterceptedService<Channel, AuthInterceptor>>;

pub fn setup_video_room_client(config: &VideoApiConfig) -> anyhow::Result<VideoRoomClient> {
	// TODO: tls
	let video_api = common::grpc::make_channel(vec![config.address.clone()], Duration::from_secs(30), None)?;

	Ok(pb::scuffle::video::v1::room_client::RoomClient::with_interceptor(
		video_api,
		AuthInterceptor {
			organization_id: config.organization_id,
			access_key: config.access_key,
			secret_key: config.secret_key,
		},
	))
}

pub fn setup_video_playback_session_client(config: &VideoApiConfig) -> anyhow::Result<VideoPlaybackSessionClient> {
	// TODO: tls
	let video_api = common::grpc::make_channel(vec![config.address.clone()], Duration::from_secs(30), None)?;

	Ok(pb::scuffle::video::v1::playback_session_client::PlaybackSessionClient::with_interceptor(
		video_api,
		AuthInterceptor {
			organization_id: config.organization_id,
			access_key: config.access_key,
			secret_key: config.secret_key,
		},
	))
}

pub fn setup_video_events_client(config: &VideoApiConfig) -> anyhow::Result<VideoEventsClient> {
	// TODO: tls
	let video_api = common::grpc::make_channel(vec![config.address.clone()], Duration::from_secs(30), None)?;

	Ok(pb::scuffle::video::v1::events_client::EventsClient::with_interceptor(
		video_api,
		AuthInterceptor {
			organization_id: config.organization_id,
			access_key: config.access_key,
			secret_key: config.secret_key,
		},
	))
}
