use std::{
    collections::{BTreeMap, HashMap},
    sync::{Arc, Weak},
};

use chrono::Utc;
use hmac::{Hmac, Mac};
use jwt::VerifyWithKey;
use pb::scuffle::video::v1::types::{
    access_token_scope::{Permission, Resource},
    AccessTokenScope,
};
use sha2::Sha256;
use tonic::{Request, Status};
use uuid::Uuid;
use video_database::{access_token::AccessToken, dataloader::IdNamePair};

use crate::global::GlobalState;

pub async fn jwt_to_access_token(
    global: &Arc<GlobalState>,
    jwt: &str,
) -> Result<AccessToken, Status> {
    let key: Hmac<Sha256> = Hmac::new_from_slice(global.config.jwt_secret.as_bytes())
        .map_err(|_| tonic::Status::internal("Failed to create HMAC key for JWT verification"))?;

    let claims: BTreeMap<String, String> = jwt
        .verify_with_key(&key)
        .map_err(|_| tonic::Status::unauthenticated("Failed to verify JWT"))?;

    // Check for existence of expiration claim / not before claim (if present, check that it's not in the future)
    let now = Utc::now();

    if let Some(exp) = claims
        .get("exp")
        .map(|exp| exp.parse::<i64>())
        .transpose()
        .map_err(|_| {
            tonic::Status::unauthenticated("JWT expiration claim is not a valid integer")
        })?
    {
        if exp < now.timestamp() {
            return Err(tonic::Status::unauthenticated("JWT has expired"));
        }
    }

    if let Some(nbf) = claims
        .get("nbf")
        .map(|nbf| nbf.parse::<i64>())
        .transpose()
        .map_err(|_| {
            tonic::Status::unauthenticated("JWT not before claim is not a valid integer")
        })?
    {
        if nbf > now.timestamp() {
            return Err(tonic::Status::unauthenticated("JWT is not yet valid"));
        }
    }

    if claims
        .get("iat")
        .map(|iat| iat.parse::<i64>())
        .ok_or_else(|| tonic::Status::unauthenticated("JWT missing issued at claim"))?
        .map_err(|_| tonic::Status::unauthenticated("JWT issued at claim is not a valid integer"))?
        > now.timestamp()
    {
        return Err(tonic::Status::unauthenticated(
            "JWT was issued in the future",
        ));
    }

    let organization_id = claims
        .get("org")
        .ok_or_else(|| tonic::Status::unauthenticated("JWT missing organization claim"))?
        .parse::<Uuid>()
        .map_err(|_| {
            tonic::Status::unauthenticated("JWT organization claim is not a valid UUID")
        })?;

    let name = claims
        .get("name")
        .ok_or_else(|| tonic::Status::unauthenticated("JWT missing name claim"))?
        .to_owned();

    let version = claims
        .get("ver")
        .ok_or_else(|| tonic::Status::unauthenticated("JWT missing version claim"))?
        .parse::<i32>()
        .map_err(|_| tonic::Status::unauthenticated("JWT version claim is not a valid integer"))?;

    let token = global
        .access_token_by_name_loader
        .load_one(IdNamePair(organization_id, name))
        .await
        .map_err(|_| tonic::Status::internal("Failed to load access token from database"))?
        .ok_or_else(|| tonic::Status::unauthenticated("JWT access token does not exist"))?;

    if token.version != version {
        return Err(tonic::Status::unauthenticated(
            "JWT version does not match access token version",
        ));
    }

    if let Some(exp) = token.expires_at {
        if exp < now {
            return Err(tonic::Status::unauthenticated(
                "JWT access token has expired",
            ));
        }
    }

    global
        .access_token_used_by_name_updater
        .load_one(IdNamePair(organization_id, token.name.clone()))
        .await
        .map_err(|_| tonic::Status::internal("Failed to update access token last used time"))?;

    Ok(token)
}

pub async fn validate_auth_request<T>(
    global: &Arc<GlobalState>,
    request: &Request<T>,
) -> Result<AccessToken, Status> {
    let auth = request
        .metadata()
        .get("authorization")
        .ok_or_else(|| Status::unauthenticated("no authorization header"))?;

    let auth = auth
        .to_str()
        .map_err(|_| Status::unauthenticated("invalid authorization header"))?;

    let auth = auth
        .strip_prefix("Bearer ")
        .ok_or_else(|| Status::unauthenticated("invalid authorization header"))?;

    jwt_to_access_token(global, auth).await
}

pub fn get_global(weak: &Weak<GlobalState>) -> Result<Arc<GlobalState>, Status> {
    weak.upgrade()
        .ok_or_else(|| Status::internal("global state was dropped"))
}

pub trait HandleInternalError<T> {
    fn to_grpc(self) -> Result<T, Status>;
}

impl<T, E: std::fmt::Display> HandleInternalError<T> for Result<T, E> {
    #[track_caller]
    fn to_grpc(self) -> Result<T, Status> {
        self.map_err(|e| {
            let location = std::panic::Location::caller();
            tracing::error!(error = %e, location = %location, "internal error");
            Status::internal("internal error".to_owned())
        })
    }
}

pub struct RequiredScope(Vec<AccessTokenScope>);

type ResourcePermission = (Resource, Permission);

impl From<ResourcePermission> for RequiredScope {
    fn from((resource, permission): ResourcePermission) -> Self {
        Self(vec![AccessTokenScope {
            resource: Some(resource.into()),
            permission: vec![permission.into()],
        }])
    }
}

impl From<Vec<ResourcePermission>> for RequiredScope {
    fn from(permissions: Vec<ResourcePermission>) -> Self {
        Self(
            permissions
                .into_iter()
                .map(|(resource, permission)| AccessTokenScope {
                    resource: Some(resource.into()),
                    permission: vec![permission.into()],
                })
                .collect(),
        )
        .optimize()
    }
}

impl From<Permission> for RequiredScope {
    fn from(permission: Permission) -> Self {
        Self(vec![AccessTokenScope {
            resource: None,
            permission: vec![permission.into()],
        }])
    }
}

impl RequiredScope {
    fn optimize(self) -> Self {
        let mut scopes = self.0;

        scopes.dedup();

        let mut scopes = scopes
            .into_iter()
            .fold(HashMap::new(), |mut map, new_scope| {
                let resource = new_scope.resource;

                let scope = map.entry(resource).or_insert_with(|| AccessTokenScope {
                    resource,
                    permission: Vec::new(),
                });

                if scope.permission.contains(&Permission::Admin.into()) {
                    return map;
                }

                if new_scope.permission.contains(&Permission::Admin.into()) {
                    scope.permission = vec![Permission::Admin.into()];
                    return map;
                }

                scope.permission.extend(new_scope.permission);

                scope.permission.sort();
                scope.permission.dedup();

                map
            });

        if let Some(global_scope) = scopes.remove(&None) {
            if global_scope.permission.contains(&Permission::Admin.into()) {
                return Self(vec![AccessTokenScope {
                    resource: None,
                    permission: vec![Permission::Admin.into()],
                }]);
            }

            scopes.iter_mut().for_each(|(_, scope)| {
                scope
                    .permission
                    .retain(|p| !global_scope.permission.contains(p));
            });

            scopes.insert(None, global_scope);
        }

        let scopes = scopes
            .into_values()
            .filter(|s| !s.permission.is_empty())
            .collect::<Vec<_>>();

        Self(scopes)
    }
}

impl From<AccessTokenScope> for RequiredScope {
    fn from(scope: AccessTokenScope) -> Self {
        Self(vec![scope])
    }
}

impl From<Vec<AccessTokenScope>> for RequiredScope {
    fn from(scopes: Vec<AccessTokenScope>) -> Self {
        Self(scopes)
    }
}

impl std::fmt::Display for RequiredScope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut permissions = Vec::new();

        for ps in &self.0 {
            let scope = ps
                .resource
                .and_then(Resource::from_i32)
                .map(|r| r.as_str_name().to_lowercase())
                .unwrap_or_else(|| "all".to_string());

            permissions.extend(
                ps.permission
                    .iter()
                    .filter_map(|p| Permission::from_i32(*p))
                    .map(|p| format!("{}:{}", scope, p.as_str_name().to_lowercase())),
            )
        }

        permissions.sort();

        permissions.join(" + ").fmt(f)
    }
}

pub trait AccessTokenExt {
    fn has_scope(&self, required: impl Into<RequiredScope>) -> Result<(), Status>;
}

impl AccessTokenExt for AccessToken {
    fn has_scope(&self, required: impl Into<RequiredScope>) -> Result<(), Status> {
        let required = required.into().optimize();

        if required.0.iter().all(|required| {
            self.scopes.iter().any(|scope| {
                // Check that the scope is for all resources (unset) or matches the resource in the required scope
                (scope.resource.is_none() || scope.resource == required.resource) &&
                // Check that the scope either has the Admin permission or has all of the required permissions
                (scope.permission.contains(&Permission::Admin.into()) || required.permission.iter().all(|p| scope.permission.contains(p)))
            })
        }) {
            Ok(())
        } else {
            Err(Status::permission_denied(format!("missing required scope: {}", required)))
        }
    }
}
