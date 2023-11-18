use std::collections::HashMap;
use std::str::FromStr;
use std::sync::{Arc, Weak};

use base64::Engine;
use pb::scuffle::video::v1::types::access_token_scope::{Permission, Resource};
use pb::scuffle::video::v1::types::{AccessTokenScope, Tags};
use tonic::{Request, Status};
use ulid::Ulid;
use uuid::Uuid;
use video_common::database::AccessToken;

use crate::global::ApiGlobal;

const MAX_TAG_COUNT: usize = 10;
const MAX_TAG_KEY_LENGTH: usize = 16;
const MAX_TAG_VALUE_LENGTH: usize = 32;
const TAG_ALPHABET: &str = r#"abcdefghijklmnopqrstuvwzyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-+?'";:[]{}"#;

pub fn validate_tags(tags: Option<&Tags>) -> Result<(), Status> {
	if let Some(tags) = tags {
		if tags.tags.len() > MAX_TAG_COUNT {
			return Err(Status::invalid_argument(format!("too many tags, max {}", MAX_TAG_COUNT)));
		}

		for (key, value) in tags.tags.iter() {
			if key.len() > MAX_TAG_KEY_LENGTH {
				return Err(Status::invalid_argument(format!(
					"tag key too long, max length {} characters",
					MAX_TAG_KEY_LENGTH
				)));
			}

			if value.len() > MAX_TAG_VALUE_LENGTH {
				return Err(Status::invalid_argument(format!(
					"tag value too long, max length {} characters",
					MAX_TAG_VALUE_LENGTH
				)));
			}

			if !key.chars().all(|c| TAG_ALPHABET.contains(c)) {
				return Err(Status::invalid_argument("tag key contains invalid characters"));
			}

			if !value.chars().all(|c| TAG_ALPHABET.contains(c)) {
				return Err(Status::invalid_argument("tag value contains invalid characters"));
			}
		}
	}

	Ok(())
}

pub async fn validate_auth_request<T, G: ApiGlobal>(
	global: &Arc<G>,
	request: &Request<T>,
	permissions: impl Into<RequiredScope>,
) -> Result<AccessToken, Status> {
	let auth = request
		.metadata()
		.get("authorization")
		.ok_or_else(|| Status::unauthenticated("no authorization header"))?;

	let auth = auth
		.to_str()
		.map_err(|_| Status::unauthenticated("invalid authorization header"))?;

	let auth = auth
		.strip_prefix("Basic ")
		.ok_or_else(|| Status::unauthenticated("invalid authorization header"))?;

	let auth = base64::engine::general_purpose::STANDARD
		.decode(auth.as_bytes())
		.map_err(|_| Status::unauthenticated("invalid authorization header"))?;

	let auth_string = String::from_utf8(auth).map_err(|_| Status::unauthenticated("invalid authorization header"))?;

	let mut parts = auth_string.splitn(2, ':');

	let access_token_id = parts
		.next()
		.ok_or_else(|| Status::unauthenticated("invalid authorization header"))?;

	let access_token_id =
		Ulid::from_str(access_token_id).map_err(|_| Status::unauthenticated("invalid authorization header"))?;

	let secret_token = parts
		.next()
		.ok_or_else(|| Status::unauthenticated("invalid authorization header"))?;

	let secret_token = Ulid::from_str(secret_token).map_err(|_| Status::unauthenticated("invalid authorization header"))?;

	let access_token = global
		.access_token_loader()
		.load(access_token_id)
		.await
		.map_err(|_| Status::internal("failed to load access token"))?
		.ok_or_else(|| Status::unauthenticated("invalid access token"))?;

	if access_token.secret_key.0 != secret_token {
		return Err(Status::unauthenticated("invalid access token"));
	}

	access_token.has_scope(permissions)?;

	Ok(access_token)
}

pub fn get_global<G: ApiGlobal>(weak: &Weak<G>) -> Result<Arc<G>, Status> {
	weak.upgrade().ok_or_else(|| Status::internal("global state was dropped"))
}

#[derive(sqlx::FromRow)]
struct TagExt {
	pub tags: sqlx::types::Json<HashMap<String, String>>,
	pub status: i32,
}

pub async fn add_tag_query<G: ApiGlobal>(
	global: &Arc<G>,
	table: &str,
	tags: &HashMap<String, String>,
	id: Ulid,
	organization_id: Option<Ulid>,
) -> Result<Option<HashMap<String, String>>, Status> {
	let mut qb = sqlx::QueryBuilder::default();

	qb.push(
		r#"WITH merged_tags AS (
    SELECT
        id,
        tags || "#,
	)
	.push_bind(sqlx::types::Json(tags))
	.push(
		r#" AS new_tags,
        CASE
        WHEN tags @> $1 THEN 1
        WHEN COUNT(jsonb_object_keys(tags || $1)) > 15 THEN 2
        ELSE 0
        END AS status
    FROM "#,
	)
	.push(table)
	.push(" WHERE id = ")
	.push_bind(Uuid::from(id));

	if let Some(organization_id) = organization_id {
		qb.push(" AND organization_id = ").push_bind(Uuid::from(organization_id));
	}

	qb.push(" GROUP BY id) UPDATE ").push(table).push(" AS t").push(
		r#"
        SET
            tags = case when merged_tags.status = 0 then merged_tags.new_tags else tags end,
            updated_at = case when merged_tags.status = 0 then now() else updated_at end
        FROM merged_tags
        WHERE t.id = merged_tags.id
        RETURNING t.tags as tags, merged_tags.status as status;
    "#,
	);

	let row: Option<TagExt> = qb
		.build_query_as()
		.fetch_optional(global.db().as_ref())
		.await
		.map_err(|err| {
			tracing::error!("failed to tag {}: {}", table, err);
			Status::internal(format!("failed to tag {}", table))
		})?;

	row.map(|row| match row.status {
		0 | 1 => Ok(row.tags.0),
		2 => Err(Status::invalid_argument("tags must not exceed 15".to_string())),
		_ => unreachable!(),
	})
	.transpose()
}

pub async fn remove_tag_query<G: ApiGlobal>(
	global: &Arc<G>,
	table: &str,
	tags: &[String],
	id: Ulid,
	organization_id: Option<Ulid>,
) -> Result<Option<HashMap<String, String>>, Status> {
	let mut qb = sqlx::QueryBuilder::default();

	qb.push(
		r#"WITH removed_tags AS (
        SELECT
            id,
            tags - "#,
	)
	.push_bind(tags)
	.push(
		r#" AS new_tags,
        CASE
            WHEN NOT tags ?| $1 THEN 1
            ELSE 0
            END AS status
        FROM "#,
	)
	.push(table)
	.push(" WHERE id = ")
	.push_bind(Uuid::from(id));

	if let Some(organization_id) = organization_id {
		qb.push(" AND organization_id = ").push_bind(Uuid::from(organization_id));
	}

	qb.push(" GROUP BY id) UPDATE ").push(table).push(" AS t").push(
		r#"
        SET
            tags = case when removed_tags.status = 0 then removed_tags.new_tags else tags end,
            updated_at = case when removed_tags.status = 0 then now() else updated_at end
        FROM removed_tags
        WHERE t.id = removed_tags.id
        RETURNING t.tags as tags, removed_tags.status as status;
    "#,
	);

	let row: Option<TagExt> = qb
		.build_query_as()
		.fetch_optional(global.db().as_ref())
		.await
		.map_err(|err| {
			tracing::error!("failed to tag {}: {}", table, err);
			Status::internal(format!("failed to tag {}", table))
		})?;

	row.map(|row| match row.status {
		0 | 1 => Ok(row.tags.0),
		2 => Err(Status::invalid_argument("tags must not exceed 15".to_string())),
		_ => unreachable!(),
	})
	.transpose()
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

		let mut scopes = scopes.into_iter().fold(HashMap::new(), |mut map, new_scope| {
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
				scope.permission.retain(|p| !global_scope.permission.contains(p));
			});

			scopes.insert(None, global_scope);
		}

		let scopes = scopes.into_values().filter(|s| !s.permission.is_empty()).collect::<Vec<_>>();

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
				.and_then(|s| Resource::try_from(s).ok())
				.map(|r| r.as_str_name().to_lowercase())
				.unwrap_or_else(|| "all".to_string());

			permissions.extend(
				ps.permission
					.iter()
					.filter_map(|p| Permission::try_from(*p).ok())
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
				// Check that the scope is for all resources (unset) or matches the resource in
				// the required scope
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
