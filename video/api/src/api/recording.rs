use crate::global::GlobalState;
use std::sync::{Arc, Weak};

use tonic::{async_trait, Request, Response, Status};

use pb::{
    ext::UlidExt,
    scuffle::video::v1::{
        recording_server::{
            Recording as RecordingServiceTrait, RecordingServer as RecordingService,
        },
        types::{
            access_token_scope::{Permission, Resource},
            Tags,
        },
        RecordingDeleteRequest, RecordingDeleteResponse, RecordingGetRequest, RecordingGetResponse,
        RecordingModifyRequest, RecordingModifyResponse, RecordingTagRequest, RecordingTagResponse,
        RecordingUntagRequest, RecordingUntagResponse,
    },
};

use super::utils::{
    add_tag_query, get_global, remove_tag_query, validate_auth_request, validate_tags,
};

type Result<T> = std::result::Result<T, Status>;

pub struct RecordingServer {
    global: Weak<GlobalState>,
}

impl RecordingServer {
    pub fn new(global: &Arc<GlobalState>) -> RecordingService<Self> {
        RecordingService::new(Self {
            global: Arc::downgrade(global),
        })
    }
}

#[async_trait]
impl RecordingServiceTrait for RecordingServer {
    async fn get(
        &self,
        _request: Request<RecordingGetRequest>,
    ) -> Result<Response<RecordingGetResponse>> {
        todo!("TODO: implement Recording::get")
    }

    async fn modify(
        &self,
        _request: Request<RecordingModifyRequest>,
    ) -> Result<Response<RecordingModifyResponse>> {
        todo!("TODO: implement Recording::modify")
    }

    async fn delete(
        &self,
        _request: Request<RecordingDeleteRequest>,
    ) -> Result<Response<RecordingDeleteResponse>> {
        todo!("TODO: implement Recording::delete")
    }

    async fn tag(
        &self,
        request: Request<RecordingTagRequest>,
    ) -> Result<Response<RecordingTagResponse>> {
        let global = get_global(&self.global)?;

        let access_token = validate_auth_request(
            &global,
            &request,
            (Resource::RecordingConfig, Permission::Modify),
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
            "recordings",
            &tags.tags,
            id,
            Some(access_token.organization_id.into()),
        )
        .await?
        .ok_or_else(|| Status::not_found("recording not found"))?;

        Ok(Response::new(RecordingTagResponse {
            tags: Some(Tags { tags: updated_tags }),
        }))
    }

    async fn untag(
        &self,
        request: Request<RecordingUntagRequest>,
    ) -> Result<Response<RecordingUntagResponse>> {
        let global = get_global(&self.global)?;

        let access_token = validate_auth_request(
            &global,
            &request,
            (Resource::RecordingConfig, Permission::Modify),
        )
        .await?;

        if request.get_ref().tags.is_empty() {
            return Err(Status::invalid_argument("tags must not be empty"));
        }

        let id = request.get_ref().id.to_ulid();

        let updated_tags = remove_tag_query(
            &global,
            "recordings",
            &request.get_ref().tags,
            id,
            Some(access_token.organization_id.into()),
        )
        .await?
        .ok_or_else(|| Status::not_found("recording config not found"))?;

        Ok(Response::new(RecordingUntagResponse {
            tags: Some(Tags { tags: updated_tags }),
        }))
    }
}
