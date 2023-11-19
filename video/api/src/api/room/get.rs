use std::sync::Arc;

use pb::ext::UlidExt;
use pb::scuffle::video::v1::types::access_token_scope::Permission;
use pb::scuffle::video::v1::types::Resource;
use pb::scuffle::video::v1::{RoomGetRequest, RoomGetResponse};
use video_common::database::{AccessToken, DatabaseTable, RoomStatus};

use crate::api::utils::{get, impl_request_scopes, QbRequest, QbResponse};
use crate::global::ApiGlobal;
use crate::ratelimit::RateLimitResource;

impl_request_scopes!(
	RoomGetRequest,
	video_common::database::Room,
	(Resource::Room, Permission::Read),
	RateLimitResource::RoomGet
);

#[async_trait::async_trait]
impl QbRequest for RoomGetRequest {
	type QueryObject = Self::Table;

	async fn build_query<G: ApiGlobal>(
		&self,
		_: &Arc<G>,
		access_token: &AccessToken,
	) -> tonic::Result<sqlx::QueryBuilder<'_, sqlx::Postgres>> {
		let mut qb = sqlx::query_builder::QueryBuilder::default();
		qb.push("SELECT * FROM ").push(Self::Table::NAME).push(" WHERE ");
		let mut seperated = qb.separated(" AND ");

		get::organization_id(&mut seperated, access_token.organization_id);
		get::ids(&mut seperated, &self.ids);

		if let Some(transcoding_config_id) = self.transcoding_config_id.as_ref() {
			seperated.push("transcoding_config_id = ");
			seperated.push_bind_unseparated(transcoding_config_id.to_uuid());
		}

		if let Some(recording_config_id) = self.recording_config_id.as_ref() {
			seperated.push("recording_config_id = ");
			seperated.push_bind_unseparated(recording_config_id.to_uuid());
		}

		if let Some(live) = self.live {
			seperated.push("status = ");
			seperated.push_bind_unseparated(if live { RoomStatus::Ready } else { RoomStatus::Offline });
		}

		if let Some(private) = self.private {
			seperated.push("private = ");
			seperated.push_bind_unseparated(private);
		}

		get::search_options(&mut seperated, self.search_options.as_ref())?;

		Ok(qb)
	}
}

impl QbResponse for RoomGetResponse {
	type Request = RoomGetRequest;

	fn from_query_object(query_objects: Vec<<Self::Request as QbRequest>::QueryObject>) -> tonic::Result<Self> {
		Ok(Self {
			rooms: query_objects
				.into_iter()
				.map(video_common::database::Room::into_proto)
				.collect(),
		})
	}
}
