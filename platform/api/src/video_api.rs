use std::time::Duration;

use anyhow::Context;
use base64::Engine;
use common::grpc::TlsSettings;
use pb::scuffle::video::v1::events_client::EventsClient;
use pb::scuffle::video::v1::playback_session_client::PlaybackSessionClient;
use pb::scuffle::video::v1::room_client::RoomClient;
use tonic::service::interceptor::InterceptedService;
use tonic::transport::Channel;
use ulid::Ulid;

use crate::config::{VideoApiConfig, VideoApiPlaybackKeypairConfig};

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

pub fn setup_video_room_client(config: &VideoApiConfig, tls: Option<TlsSettings>) -> anyhow::Result<VideoRoomClient> {
	let video_api = common::grpc::make_channel(vec![config.address.clone()], Duration::from_secs(30), tls)?;

	Ok(pb::scuffle::video::v1::room_client::RoomClient::with_interceptor(
		video_api,
		AuthInterceptor {
			organization_id: config.organization_id,
			access_key: config.access_key,
			secret_key: config.secret_key,
		},
	))
}

pub fn setup_video_playback_session_client(
	config: &VideoApiConfig,
	tls: Option<TlsSettings>,
) -> anyhow::Result<VideoPlaybackSessionClient> {
	let video_api = common::grpc::make_channel(vec![config.address.clone()], Duration::from_secs(30), tls)?;

	Ok(
		pb::scuffle::video::v1::playback_session_client::PlaybackSessionClient::with_interceptor(
			video_api,
			AuthInterceptor {
				organization_id: config.organization_id,
				access_key: config.access_key,
				secret_key: config.secret_key,
			},
		),
	)
}

pub fn setup_video_events_client(config: &VideoApiConfig, tls: Option<TlsSettings>) -> anyhow::Result<VideoEventsClient> {
	let video_api = common::grpc::make_channel(vec![config.address.clone()], Duration::from_secs(30), tls)?;

	Ok(pb::scuffle::video::v1::events_client::EventsClient::with_interceptor(
		video_api,
		AuthInterceptor {
			organization_id: config.organization_id,
			access_key: config.access_key,
			secret_key: config.secret_key,
		},
	))
}

pub fn load_playback_keypair_private_key(
	pbkp_config: &VideoApiPlaybackKeypairConfig,
) -> anyhow::Result<jwt::asymmetric::AsymmetricKeyWithDigest<jwt::asymmetric::SigningKey>> {
	let key_string = std::fs::read_to_string(&pbkp_config.private_key).with_context(|| {
		format!(
			"failed to read video api playback keypair private key from {}",
			pbkp_config.private_key.display()
		)
	})?;
	let key = jwt::asymmetric::PrivateKey::from_pem(&key_string)
		.context("failed to parse video api playback keypair private key")?
		.into_ec384()
		.ok()
		.context("video api playback keypair private key is not EC384")?;
	Ok(jwt::asymmetric::AsymmetricKeyWithDigest::new(
		jwt::asymmetric::SigningKey::from_ec384(key),
	))
}

pub async fn request_deduplicated_viewer_count(
	client: &mut VideoPlaybackSessionClient,
	room_id: Ulid,
) -> tonic::Result<i32> {
	let res = client
		.count(pb::scuffle::video::v1::PlaybackSessionCountRequest {
			filter: Some(pb::scuffle::video::v1::playback_session_count_request::Filter::Target(
				pb::scuffle::video::v1::types::PlaybackSessionTarget {
					target: Some(pb::scuffle::video::v1::types::playback_session_target::Target::RoomId(
						room_id.into(),
					)),
				},
			)),
		})
		.await?;

	Ok(res.into_inner().deduplicated_count as i32) //should be safe to cast
}
