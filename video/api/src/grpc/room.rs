use crate::global::GlobalState;
use std::{
    collections::HashMap,
    sync::{Arc, Weak},
};

use sqlx::FromRow;
use tonic::{async_trait, Request, Response, Status};

use pb::scuffle::video::v1::{
    room_server::{Room as RoomServiceTrait, RoomServer as RoomService},
    types::{
        access_token_scope::{Permission, Resource},
        ModifyMode,
    },
    RoomDeleteRequest, RoomDeleteResponse, RoomDisconnectRequest, RoomDisconnectResponse,
    RoomGetRequest, RoomGetResponse, RoomModifyRequest, RoomModifyResponse, RoomResetKeyRequest,
    RoomResetKeyResponse,
};
use video_database::{dataloader::IdNamePair, room::Room};

use super::utils::{get_global, validate_auth_request, AccessTokenExt, HandleInternalError};

type Result<T> = std::result::Result<T, Status>;

mod utils;

#[cfg(test)]
mod tests;

/// Room allows you to create, update, get, disconnect, reset key, and delete
/// rooms. A room is a live stream that can be published to and viewed. Rooms can
/// be configured with a transcoding config, recording config, to define how the
/// stream is transcoded and recorded.
pub struct RoomServer {
    global: Weak<GlobalState>,
}

impl RoomServer {
    pub fn new(global: &Arc<GlobalState>) -> RoomService<Self> {
        RoomService::new(Self {
            global: Arc::downgrade(global),
        })
    }
}

#[async_trait]
impl RoomServiceTrait for RoomServer {
    /// Modify allows you to create a new room or update an existing room.
    async fn modify(
        &self,
        request: Request<RoomModifyRequest>,
    ) -> Result<Response<RoomModifyResponse>> {
        let global = get_global(&self.global)?;

        let access_token = validate_auth_request(&global, &request).await?;

        access_token.has_scope((Resource::Room, Permission::Modify))?;

        let request = request.into_inner();

        let mut query_builder = utils::room_modify_query(&request, &access_token)?;
        let room = Room::from_row(
            &query_builder
                .build()
                .fetch_one(global.db.as_ref())
                .await
                .map_err(|e| {
                    if let Some(e) = e.as_database_error() {
                        if e.is_unique_violation() {
                            // Is name unique violation
                            return Status::already_exists("room name already exists");
                        }

                        if let Some(constraint) = e.constraint() {
                            match constraint {
                                "fk_room_transcoding_config" => {
                                    return Status::not_found("transcoding config not found");
                                }
                                "fk_room_recording_config" => {
                                    return Status::not_found("recording config not found");
                                }
                                "fk_room_organization" => {
                                    return Status::not_found("organization not found");
                                }
                                _ => {}
                            }
                        }
                    }

                    tracing::error!(error = %e, "failed to modify room");
                    Status::internal("failed to modify room")
                })?,
        )
        .to_grpc()?;

        let created = match request.mode() {
            ModifyMode::Create => true,
            ModifyMode::Update => false,
            ModifyMode::Upsert => room.created_at == room.updated_at,
        };

        Ok(Response::new(RoomModifyResponse {
            room: Some(room.into_proto()),
            created,
        }))
    }

    /// Get allows you to get rooms.
    async fn get(&self, request: Request<RoomGetRequest>) -> Result<Response<RoomGetResponse>> {
        let global = get_global(&self.global)?;

        let access_token = validate_auth_request(&global, &request).await?;

        access_token.has_scope((Resource::Room, Permission::Read))?;

        let request = request.into_inner();

        let mut query_builder = utils::room_get_query(&request, &access_token)?;

        let rooms = query_builder
            .build()
            .fetch_all(global.db.as_ref())
            .await
            .to_grpc()?
            .iter()
            .map(|r| Room::from_row(r).map(|r| r.into_proto()))
            .collect::<sqlx::Result<Vec<_>>>()
            .to_grpc()?;

        Ok(Response::new(RoomGetResponse { rooms }))
    }

    /// Disconnect allows you to disconnect a currently live room.
    async fn disconnect(
        &self,
        request: Request<RoomDisconnectRequest>,
    ) -> Result<Response<RoomDisconnectResponse>> {
        todo!("TODO: implement Room::disconnect")
    }

    /// ResetKey allows you to reset the key for a room.
    async fn reset_key(
        &self,
        request: Request<RoomResetKeyRequest>,
    ) -> Result<Response<RoomResetKeyResponse>> {
        let global = get_global(&self.global)?;

        let access_token = validate_auth_request(&global, &request).await?;

        access_token.has_scope((Resource::Room, Permission::Modify))?;

        let request = request.into_inner();

        let mut query_builder = utils::room_reset_key_query(&request, &access_token)?;

        let rooms = query_builder
            .build()
            .fetch_all(global.db.as_ref())
            .await
            .to_grpc()?
            .iter()
            .map(Room::from_row)
            .collect::<sqlx::Result<Vec<_>>>()
            .to_grpc()?;

        Ok(Response::new(RoomResetKeyResponse {
            room_keys: rooms
                .into_iter()
                .map(|r| (r.name, r.stream_key))
                .collect::<HashMap<_, _>>(),
        }))
    }

    /// Delete allows you to delete rooms.
    async fn delete(
        &self,
        _request: Request<RoomDeleteRequest>,
    ) -> Result<Response<RoomDeleteResponse>> {
        todo!("TODO: implement Room::delete")
    }
}
