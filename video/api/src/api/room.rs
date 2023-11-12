use crate::global::ApiGlobal;
use std::sync::{Arc, Weak};

use tonic::{async_trait, Request, Response, Status};

use pb::{
    ext::UlidExt,
    scuffle::video::v1::{
        room_server::{Room as RoomServiceTrait, RoomServer as RoomService},
        types::{
            access_token_scope::{Permission, Resource},
            Tags,
        },
        RoomCreateRequest, RoomCreateResponse, RoomDeleteRequest, RoomDeleteResponse,
        RoomDisconnectRequest, RoomDisconnectResponse, RoomGetRequest, RoomGetResponse,
        RoomModifyRequest, RoomModifyResponse, RoomResetKeyRequest, RoomResetKeyResponse,
        RoomTagRequest, RoomTagResponse, RoomUntagRequest, RoomUntagResponse,
    },
};

use super::utils::{
    add_tag_query, get_global, remove_tag_query, validate_auth_request, validate_tags,
};

type Result<T> = std::result::Result<T, Status>;

pub struct RoomServer<G: ApiGlobal> {
    global: Weak<G>,
}

impl<G: ApiGlobal> RoomServer<G> {
    pub fn new(global: &Arc<G>) -> RoomService<Self> {
        RoomService::new(Self {
            global: Arc::downgrade(global),
        })
    }
}

#[async_trait]
impl<G: ApiGlobal> RoomServiceTrait for RoomServer<G> {
    async fn get(&self, _request: Request<RoomGetRequest>) -> Result<Response<RoomGetResponse>> {
        todo!("TODO: implement Room::get")
    }

    async fn create(
        &self,
        _request: Request<RoomCreateRequest>,
    ) -> Result<Response<RoomCreateResponse>> {
        todo!("TODO: implement Room::create")
    }

    async fn modify(
        &self,
        _request: Request<RoomModifyRequest>,
    ) -> Result<Response<RoomModifyResponse>> {
        todo!("TODO: implement Room::modify")
    }

    async fn delete(
        &self,
        _request: Request<RoomDeleteRequest>,
    ) -> Result<Response<RoomDeleteResponse>> {
        todo!("TODO: implement Room::delete")
    }

    async fn disconnect(
        &self,
        _request: Request<RoomDisconnectRequest>,
    ) -> Result<Response<RoomDisconnectResponse>> {
        todo!("TODO: implement Room::disconnect")
    }

    async fn reset_key(
        &self,
        _request: Request<RoomResetKeyRequest>,
    ) -> Result<Response<RoomResetKeyResponse>> {
        todo!("TODO: implement Room::reset_key")
    }

    async fn tag(&self, request: Request<RoomTagRequest>) -> Result<Response<RoomTagResponse>> {
        let global = get_global(&self.global)?;

        let access_token =
            validate_auth_request(&global, &request, (Resource::Room, Permission::Modify)).await?;

        let Some(tags) = request.get_ref().tags.as_ref() else {
            return Err(Status::invalid_argument("tags must be provided"));
        };

        if tags.tags.is_empty() {
            return Err(Status::invalid_argument("tags must not be empty"));
        }

        validate_tags(Some(tags))?;

        let id = request.get_ref().id.to_ulid();

        let updated_tags = add_tag_query(
            &global,
            "rooms",
            &tags.tags,
            id,
            Some(access_token.organization_id.into()),
        )
        .await?
        .ok_or_else(|| Status::not_found("room not found"))?;

        Ok(Response::new(RoomTagResponse {
            tags: Some(Tags { tags: updated_tags }),
        }))
    }

    async fn untag(
        &self,
        request: Request<RoomUntagRequest>,
    ) -> Result<Response<RoomUntagResponse>> {
        let global = get_global(&self.global)?;

        let access_token =
            validate_auth_request(&global, &request, (Resource::Room, Permission::Modify)).await?;

        if request.get_ref().tags.is_empty() {
            return Err(Status::invalid_argument("tags must not be empty"));
        }

        let id = request.get_ref().id.to_ulid();

        let updated_tags = remove_tag_query(
            &global,
            "rooms",
            &request.get_ref().tags,
            id,
            Some(access_token.organization_id.into()),
        )
        .await?
        .ok_or_else(|| Status::not_found("room not found"))?;

        Ok(Response::new(RoomUntagResponse {
            tags: Some(Tags { tags: updated_tags }),
        }))
    }
}
