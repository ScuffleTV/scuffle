use crate::global::ApiGlobal;
use std::sync::{Arc, Weak};

use tonic::{async_trait, Request, Response, Status};

use pb::{
    ext::UlidExt,
    scuffle::video::v1::{
        s3_bucket_server::{S3Bucket as S3BucketServiceTrait, S3BucketServer as S3BucketService},
        types::{
            access_token_scope::{Permission, Resource},
            Tags,
        },
        S3BucketCreateRequest, S3BucketCreateResponse, S3BucketDeleteRequest,
        S3BucketDeleteResponse, S3BucketGetRequest, S3BucketGetResponse, S3BucketModifyRequest,
        S3BucketModifyResponse, S3BucketTagRequest, S3BucketTagResponse, S3BucketUntagRequest,
        S3BucketUntagResponse,
    },
};

use super::utils::{
    add_tag_query, get_global, remove_tag_query, validate_auth_request, validate_tags,
};

type Result<T> = std::result::Result<T, Status>;

pub struct S3BucketServer<G: ApiGlobal> {
    global: Weak<G>,
}

impl<G: ApiGlobal> S3BucketServer<G> {
    pub fn new(global: &Arc<G>) -> S3BucketService<Self> {
        S3BucketService::new(Self {
            global: Arc::downgrade(global),
        })
    }
}

#[async_trait]
impl<G: ApiGlobal> S3BucketServiceTrait for S3BucketServer<G> {
    async fn get(
        &self,
        _request: Request<S3BucketGetRequest>,
    ) -> Result<Response<S3BucketGetResponse>> {
        todo!("TODO: implement S3Bucket::get")
    }

    async fn create(
        &self,
        _request: Request<S3BucketCreateRequest>,
    ) -> Result<Response<S3BucketCreateResponse>> {
        todo!("TODO: implement S3Bucket::create")
    }

    async fn modify(
        &self,
        _request: Request<S3BucketModifyRequest>,
    ) -> Result<Response<S3BucketModifyResponse>> {
        todo!("TODO: implement S3Bucket::modify")
    }

    async fn delete(
        &self,
        _request: Request<S3BucketDeleteRequest>,
    ) -> Result<Response<S3BucketDeleteResponse>> {
        todo!("TODO: implement S3Bucket::delete")
    }

    async fn tag(
        &self,
        request: Request<S3BucketTagRequest>,
    ) -> Result<Response<S3BucketTagResponse>> {
        let global = get_global(&self.global)?;

        let access_token =
            validate_auth_request(&global, &request, (Resource::S3Bucket, Permission::Modify))
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
            "s3_buckets",
            &tags.tags,
            id,
            Some(access_token.organization_id.into()),
        )
        .await?
        .ok_or_else(|| Status::not_found("room not found"))?;

        Ok(Response::new(S3BucketTagResponse {
            tags: Some(Tags { tags: updated_tags }),
        }))
    }

    async fn untag(
        &self,
        request: Request<S3BucketUntagRequest>,
    ) -> Result<Response<S3BucketUntagResponse>> {
        let global = get_global(&self.global)?;

        let access_token =
            validate_auth_request(&global, &request, (Resource::S3Bucket, Permission::Modify))
                .await?;

        if request.get_ref().tags.is_empty() {
            return Err(Status::invalid_argument("tags must not be empty"));
        }

        let id = request.get_ref().id.to_ulid();

        let updated_tags = remove_tag_query(
            &global,
            "s3_buckets",
            &request.get_ref().tags,
            id,
            Some(access_token.organization_id.into()),
        )
        .await?
        .ok_or_else(|| Status::not_found("recording config not found"))?;

        Ok(Response::new(S3BucketUntagResponse {
            tags: Some(Tags { tags: updated_tags }),
        }))
    }
}
