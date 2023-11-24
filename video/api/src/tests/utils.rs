use std::collections::HashMap;
use std::ops::Add;
use std::sync::Arc;

use common::context::Handler;
use pb::scuffle::video::v1::types::{access_token_scope, AccessTokenScope};
use ulid::Ulid;
use video_common::database::AccessToken;

use super::global::{mock_global_state, GlobalState};
use crate::config::ApiConfig;
use crate::global::ApiGlobal;

pub async fn create_organization(global: &Arc<impl ApiGlobal>) -> video_common::database::Organization {
	sqlx::query_as("INSERT INTO organizations (id, name, updated_at, tags) VALUES ($1, $2, $3, $4) RETURNING *")
		.bind(common::database::Ulid(Ulid::new()))
		.bind("test")
		.bind(chrono::Utc::now())
		.bind(sqlx::types::Json(std::collections::HashMap::<String, String>::default()))
		.fetch_one(global.db().as_ref())
		.await
		.expect("Failed to create organization")
}

pub async fn create_access_token(
	global: &Arc<impl ApiGlobal>,
	organization_id: &common::database::Ulid,
	scopes: Vec<common::database::Protobuf<AccessTokenScope>>,
	tags: std::collections::HashMap<String, String>,
) -> video_common::database::AccessToken {
	sqlx::query_as(
        "INSERT INTO access_tokens (id, organization_id, secret_token, last_active_at, updated_at, expires_at, scopes, tags) VALUES ($1, $2, $3, $4, $5, $6, $7, $8) RETURNING *",
    ).bind(common::database::Ulid(Ulid::new()))
    .bind(organization_id)
    .bind(common::database::Ulid(Ulid::new()))
    .bind(chrono::Utc::now())
    .bind(chrono::Utc::now())
    .bind(chrono::Utc::now().add(chrono::Duration::days(1)))
    .bind(scopes)
    .bind(sqlx::types::Json(tags))
    .fetch_one(global.db().as_ref())
    .await
    .expect("Failed to create access token")
}

// Shared setup function
pub async fn setup(config: ApiConfig) -> (Arc<GlobalState>, Handler, AccessToken) {
	let (global, handler) = mock_global_state(config).await;
	let org = create_organization(&global).await;
	let access_token = create_access_token(
		&global,
		&org.id,
		vec![common::database::Protobuf(AccessTokenScope {
			permission: vec![access_token_scope::Permission::Admin.into()],
			resource: None,
		})],
		HashMap::new(),
	)
	.await;
	(global, handler, access_token)
}

// Shared teardown function
pub async fn teardown(global: Arc<GlobalState>, handler: Handler) {
	drop(global);
	handler.cancel().await;
}
