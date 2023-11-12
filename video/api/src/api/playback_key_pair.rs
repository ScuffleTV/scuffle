use crate::global::ApiGlobal;
use std::sync::{Arc, Weak};

use tonic::{async_trait, Request, Response, Status};

use pb::{
    ext::UlidExt,
    scuffle::video::v1::{
        playback_key_pair_server::{
            PlaybackKeyPair as PlaybackKeyPairServiceTrait,
            PlaybackKeyPairServer as PlaybackKeyPairService,
        },
        types::{
            access_token_scope::{Permission, Resource},
            Tags,
        },
        PlaybackKeyPairCreateRequest, PlaybackKeyPairCreateResponse, PlaybackKeyPairDeleteRequest,
        PlaybackKeyPairDeleteResponse, PlaybackKeyPairGetRequest, PlaybackKeyPairGetResponse,
        PlaybackKeyPairModifyRequest, PlaybackKeyPairModifyResponse, PlaybackKeyPairTagRequest,
        PlaybackKeyPairTagResponse, PlaybackKeyPairUntagRequest, PlaybackKeyPairUntagResponse,
    },
};

use super::utils::{
    add_tag_query, get_global, remove_tag_query, validate_auth_request, validate_tags,
};

type Result<T> = std::result::Result<T, Status>;

pub struct PlaybackKeyPairServer<G: ApiGlobal> {
    global: Weak<G>,
}

impl<G: ApiGlobal> PlaybackKeyPairServer<G> {
    pub fn new(global: &Arc<G>) -> PlaybackKeyPairService<Self> {
        PlaybackKeyPairService::new(Self {
            global: Arc::downgrade(global),
        })
    }
}

#[async_trait]
impl<G: ApiGlobal> PlaybackKeyPairServiceTrait for PlaybackKeyPairServer<G> {
    async fn get(
        &self,
        _request: Request<PlaybackKeyPairGetRequest>,
    ) -> Result<Response<PlaybackKeyPairGetResponse>> {
        todo!("TODO: implement PlaybackKeyPair::get")
    }

    async fn create(
        &self,
        _request: Request<PlaybackKeyPairCreateRequest>,
    ) -> Result<Response<PlaybackKeyPairCreateResponse>> {
        todo!("TODO: implement PlaybackKeyPair::create")
    }

    async fn modify(
        &self,
        _request: Request<PlaybackKeyPairModifyRequest>,
    ) -> Result<Response<PlaybackKeyPairModifyResponse>> {
        todo!("TODO: implement PlaybackKeyPair::modify")
    }

    async fn delete(
        &self,
        _request: Request<PlaybackKeyPairDeleteRequest>,
    ) -> Result<Response<PlaybackKeyPairDeleteResponse>> {
        todo!("TODO: implement PlaybackKeyPair::delete")
    }

    async fn tag(
        &self,
        request: Request<PlaybackKeyPairTagRequest>,
    ) -> Result<Response<PlaybackKeyPairTagResponse>> {
        let global = get_global(&self.global)?;

        let access_token = validate_auth_request(
            &global,
            &request,
            (Resource::PlaybackKeyPair, Permission::Modify),
        )
        .await?;

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
            "playback_key_pairs",
            &tags.tags,
            id,
            Some(access_token.organization_id.into()),
        )
        .await?
        .ok_or_else(|| Status::not_found("playback key pair not found"))?;

        Ok(Response::new(PlaybackKeyPairTagResponse {
            tags: Some(Tags { tags: updated_tags }),
        }))
    }

    async fn untag(
        &self,
        request: Request<PlaybackKeyPairUntagRequest>,
    ) -> Result<Response<PlaybackKeyPairUntagResponse>> {
        let global = get_global(&self.global)?;

        let access_token = validate_auth_request(
            &global,
            &request,
            (Resource::PlaybackKeyPair, Permission::Modify),
        )
        .await?;

        if request.get_ref().tags.is_empty() {
            return Err(Status::invalid_argument("tags must not be empty"));
        }

        let id = request.get_ref().id.to_ulid();

        let updated_tags = remove_tag_query(
            &global,
            "playback_key_pairs",
            &request.get_ref().tags,
            id,
            Some(access_token.organization_id.into()),
        )
        .await?
        .ok_or_else(|| Status::not_found("playback key pair not found"))?;

        Ok(Response::new(PlaybackKeyPairUntagResponse {
            tags: Some(Tags { tags: updated_tags }),
        }))
    }
}
