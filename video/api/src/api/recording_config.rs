use crate::global::GlobalState;
use std::sync::{Arc, Weak};

use tonic::{async_trait, Request, Response, Status};

use pb::{
    ext::UlidExt,
    scuffle::video::v1::{
        recording_config_server::{
            RecordingConfig as RecordingConfigServiceTrait,
            RecordingConfigServer as RecordingConfigService,
        },
        types::{
            access_token_scope::{Permission, Resource},
            Tags,
        },
        RecordingConfigCreateRequest, RecordingConfigCreateResponse, RecordingConfigDeleteRequest,
        RecordingConfigDeleteResponse, RecordingConfigGetRequest, RecordingConfigGetResponse,
        RecordingConfigModifyRequest, RecordingConfigModifyResponse, RecordingConfigTagRequest,
        RecordingConfigTagResponse, RecordingConfigUntagRequest, RecordingConfigUntagResponse,
    },
};

use super::utils::{
    add_tag_query, get_global, remove_tag_query, validate_auth_request, validate_tags,
};

type Result<T> = std::result::Result<T, Status>;

pub struct RecordingConfigServer {
    global: Weak<GlobalState>,
}

impl RecordingConfigServer {
    pub fn new(global: &Arc<GlobalState>) -> RecordingConfigService<Self> {
        RecordingConfigService::new(Self {
            global: Arc::downgrade(global),
        })
    }
}

#[async_trait]
impl RecordingConfigServiceTrait for RecordingConfigServer {
    async fn get(
        &self,
        _request: Request<RecordingConfigGetRequest>,
    ) -> Result<Response<RecordingConfigGetResponse>> {
        todo!("TODO: implement RecordingConfig::get")
    }

    async fn create(
        &self,
        _request: Request<RecordingConfigCreateRequest>,
    ) -> Result<Response<RecordingConfigCreateResponse>> {
        todo!("TODO: implement RecordingConfig::create")
    }

    async fn modify(
        &self,
        _request: Request<RecordingConfigModifyRequest>,
    ) -> Result<Response<RecordingConfigModifyResponse>> {
        todo!("TODO: implement RecordingConfig::modify")
    }

    async fn delete(
        &self,
        _request: Request<RecordingConfigDeleteRequest>,
    ) -> Result<Response<RecordingConfigDeleteResponse>> {
        todo!("TODO: implement RecordingConfig::delete")
    }

    async fn tag(
        &self,
        request: Request<RecordingConfigTagRequest>,
    ) -> Result<Response<RecordingConfigTagResponse>> {
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
            "recording_configs",
            &tags.tags,
            id,
            Some(access_token.organization_id.into()),
        )
        .await?
        .ok_or_else(|| Status::not_found("recording config not found"))?;

        Ok(Response::new(RecordingConfigTagResponse {
            tags: Some(Tags { tags: updated_tags }),
        }))
    }

    async fn untag(
        &self,
        request: Request<RecordingConfigUntagRequest>,
    ) -> Result<Response<RecordingConfigUntagResponse>> {
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
            "recording_configs",
            &request.get_ref().tags,
            id,
            Some(access_token.organization_id.into()),
        )
        .await?
        .ok_or_else(|| Status::not_found("recording config not found"))?;

        Ok(Response::new(RecordingConfigUntagResponse {
            tags: Some(Tags { tags: updated_tags }),
        }))
    }
}
