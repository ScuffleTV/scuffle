use async_graphql::Context;
use ulid::Ulid;

use super::error::{GqlError, Result};
use super::ext::ContextExt;
use crate::database::RolePermission;
use crate::global::ApiGlobal;

// This can't be replaced by async_graphql's field guards because of this: https://github.com/async-graphql/async-graphql/issues/1398
// I don't see a better alternative than doing this for now.
pub async fn auth_guard<T, G: ApiGlobal>(
	ctx: &Context<'_>,
	field_name: &'static str,
	field_value: T,
	user_id: Ulid,
) -> Result<T> {
	let global = ctx.get_global::<G>();

	let request_context = ctx.get_req_context();

	let auth = request_context.auth(global).await?;

	if let Some(auth) = auth {
		if Ulid::from(auth.session.user_id) == user_id || auth.user_permissions.has_permission(RolePermission::Admin) {
			return Ok(field_value);
		}
	}

	Err(GqlError::Unauthorized { field: field_name }.into())
}
