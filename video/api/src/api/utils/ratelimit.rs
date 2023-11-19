use std::str::FromStr;
use std::sync::{Arc, Weak};

use base64::Engine;
use common::ratelimiter::{RateLimitResponse, RateLimiterOptions};
use fred::interfaces::KeysInterface;
use futures_util::Future;
use tonic::metadata::{AsciiMetadataValue, MetadataMap};
use tonic::{Response, Status};
use ulid::Ulid;
use video_common::database::AccessToken;

use super::{AccessTokenExt, RequiredScope};
use crate::config::ApiConfig;
use crate::global::ApiGlobal;
use crate::ratelimit::RateLimitResource;

const RATELIMIT_BANNED_HEADER: &str = "X-RateLimit-Banned";
const RATELIMIT_RESET_HEADER: &str = "X-RateLimit-Reset";
const RATELIMIT_REMAINING_HEADER: &str = "X-RateLimit-Remaining";

pub async fn ratelimit_scoped<G: ApiGlobal, T, F: Future<Output = tonic::Result<Response<T>>>>(
	global: &Arc<G>,
	organization_id: Ulid,
	resource: RateLimitResource,
	scoped_fn: impl FnOnce() -> F,
) -> tonic::Result<Response<T>> {
	let config = global.config::<ApiConfig>();

	let ratelimit_rules = config
		.rate_limit_rules
		.rules
		.get(&resource)
		.unwrap_or(&config.rate_limit_rules.default);

	if ratelimit_rules.cost == 0 {
		return scoped_fn().await;
	}

	if ratelimit_rules.cost > ratelimit_rules.quota || ratelimit_rules.quota == 0 {
		return Err(Status::resource_exhausted("resource disabled"));
	}

	let options = RateLimiterOptions {
		cost: ratelimit_rules.cost,
		quota: ratelimit_rules.quota,
		quota_reset_seconds: ratelimit_rules.quota_reset_seconds,
		exceeded_limit: config.rate_limit_rules.banned_exceeded.exceeded_limit,
		exceeded_reset_seconds: config.rate_limit_rules.banned_exceeded.exceeded_reset_seconds,
		banned_reset_seconds: config.rate_limit_rules.banned_exceeded.banned_reset_seconds,

		limit_key: format!("{resource}"),
		banned_key: "banned".to_string(),
		exceeded_key: "exceeded".to_string(),
		namespace: format!("{{ratelimit:organization:{organization_id}}}"),
	};

	let mut resp = ratelimit(global, &options).await?;

	match scoped_fn().await {
		Ok(mut v) => {
			if let Some(reset) = resp.reset {
				v.metadata_mut().insert(RATELIMIT_RESET_HEADER, reset.as_secs().into());
			}

			v.metadata_mut().insert(RATELIMIT_REMAINING_HEADER, resp.remaining.into());

			Ok(v)
		}
		Err(mut err) => {
			if let Some(reset) = resp.reset {
				err.metadata_mut().insert(RATELIMIT_RESET_HEADER, reset.as_secs().into());
			}

			let restore_amount = ratelimit_rules.cost.saturating_sub(ratelimit_rules.failed_cost);

			if restore_amount > 0 {
				let redis = global.redis();
				resp.remaining = redis
					.decr_by(
						format!("{}:{}", options.namespace, options.limit_key),
						(i64::from(options.cost) - 1).max(1),
					)
					.await
					.unwrap_or(0);
			}

			err.metadata_mut().insert(RATELIMIT_REMAINING_HEADER, resp.remaining.into());

			Err(err)
		}
	}
}

async fn ratelimit<G: ApiGlobal>(global: &Arc<G>, options: &RateLimiterOptions) -> tonic::Result<RateLimitResponse> {
	let redis = global.redis();

	let resp = common::ratelimiter::ratelimit(redis.as_ref(), options).await.map_err(|err| {
		tracing::error!(err = %err, "failed to rate limit");
		Status::internal("Unable to process request")
	})?;

	if resp.banned || resp.remaining == -1 {
		let mut status = Status::resource_exhausted("rate limit exceeded");

		if let Some(reset) = resp.reset {
			status.metadata_mut().insert(RATELIMIT_RESET_HEADER, reset.as_secs().into());
		}

		if resp.banned {
			status
				.metadata_mut()
				.insert(RATELIMIT_BANNED_HEADER, AsciiMetadataValue::from_static("true"));
		}

		status
			.metadata_mut()
			.insert(RATELIMIT_REMAINING_HEADER, AsciiMetadataValue::from_static("0"));

		Err(status)
	} else {
		Ok(resp)
	}
}

pub fn get_global<G: ApiGlobal>(weak: &Weak<G>) -> tonic::Result<Arc<G>> {
	weak.upgrade().ok_or_else(|| Status::internal("global state was dropped"))
}

pub trait TonicRequest {
	type Table;

	fn permission_scope(&self) -> RequiredScope;
	fn ratelimit_scope(&self) -> RateLimitResource;
}

pub async fn validate_auth_request<G: ApiGlobal>(
	global: &Arc<G>,
	metadata: &MetadataMap,
	permissions: impl Into<RequiredScope>,
) -> tonic::Result<AccessToken> {
	let auth = metadata
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
		.map_err(|()| Status::internal("failed to load access token"))?
		.ok_or_else(|| Status::unauthenticated("invalid access token"))?;

	if access_token.secret_key.0 != secret_token {
		return Err(Status::unauthenticated("invalid access token"));
	}

	access_token.has_scope(permissions)?;

	Ok(access_token)
}

macro_rules! scope_ratelimit {
	($self:ident, $request:ident, $global:ident, $access_token:ident, $logic:expr) => {
		let $global = crate::api::utils::get_global(&$self.global)?;
		let $access_token = crate::api::utils::validate_auth_request(
			&$global,
			$request.metadata(),
			crate::api::utils::TonicRequest::permission_scope($request.get_ref()),
		)
		.await?;
		return crate::api::utils::ratelimit::ratelimit_scoped(
			&$global,
			$access_token.organization_id.0,
			crate::api::utils::TonicRequest::ratelimit_scope($request.get_ref()),
			$logic,
		)
		.await;
	};
}

pub(crate) use scope_ratelimit;
