use std::str::FromStr;
use std::sync::Arc;

use bytes::Bytes;
use chrono::{DateTime, Datelike};
use futures_util::future::BoxFuture;
use hex::ToHex;
use hmac::Mac;
use hyper::body::HttpBody;
use sha2::Digest;
use tonic::body::BoxBody;
use tonic::Status;
use ulid::Ulid;
use video_common::database::AccessToken;

use super::{AccessTokenExt, RequiredScope};
use crate::global::ApiGlobal;

const X_SCUF_DATE: &str = "x-scuf-date";
const X_SCUF_DATE_FMT: &str = "%Y%m%dT%H%M%SZ";
const X_SCUF_CONTENT_SHA256: &str = "x-scuf-content-sha256";

pub struct AuthMiddleware<G>(std::marker::PhantomData<G>);

impl<G> Default for AuthMiddleware<G> {
	fn default() -> Self {
		Self(std::marker::PhantomData)
	}
}

impl<G> Clone for AuthMiddleware<G> {
	fn clone(&self) -> Self {
		*self
	}
}

impl<G> Copy for AuthMiddleware<G> {}

impl<G> std::fmt::Debug for AuthMiddleware<G> {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("AuthMiddleware").finish()
	}
}

impl<S, G> tower::Layer<S> for AuthMiddleware<G> {
	type Service = AuthSvc<S, G>;

	fn layer(&self, inner: S) -> Self::Service {
		AuthSvc {
			inner,
			_marker: std::marker::PhantomData,
		}
	}
}

pub struct AuthSvc<S, G> {
	inner: S,
	_marker: std::marker::PhantomData<G>,
}

impl<S, G> Clone for AuthSvc<S, G>
where
	S: Clone,
{
	fn clone(&self) -> Self {
		Self {
			inner: self.inner.clone(),
			_marker: self._marker,
		}
	}
}

impl<S: Copy, G> Copy for AuthSvc<S, G> {}

impl<S, G> std::fmt::Debug for AuthSvc<S, G>
where
	S: std::fmt::Debug,
{
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		f.debug_struct("AuthService").field("inner", &self.inner).finish()
	}
}

impl<S, G> tower::Service<http::Request<hyper::Body>> for AuthSvc<S, G>
where
	S: tower::Service<http::Request<hyper::Body>, Response = http::Response<BoxBody>> + Clone + Send + 'static,
	G: ApiGlobal,
	S::Error: From<Status>,
	S::Future: Send + 'static,
{
	type Error = S::Error;
	type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;
	type Response = S::Response;

	fn call(&mut self, req: http::Request<hyper::Body>) -> Self::Future {
		let mut inner = self.inner.clone();

		Box::pin(async move {
			let req = auth_middleware_impl::<G>(req).await?;

			inner.call(req).await
		})
	}

	fn poll_ready(&mut self, cx: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), Self::Error>> {
		self.inner.poll_ready(cx)
	}
}

/// This middleware is used to authenticate requests using the SCUF-HMAC-SHA256
/// scheme. It will verify the signature of the request and load the access
/// token from the database. The access token will be stored in the request
/// extensions.
///
/// The SCUF-HMAC-SHA256 scheme is defined as follows:
/// authorization: SCUF-HMAC-SHA256
/// Credential={organization_id}/{access_token_id}/{date},
/// SignedHeaders={signed_headers}, Signature={signature}
///
/// Each signed header is separated by a comma.
///
/// The Carnonical Request is defined as follows:
/// {method}\n
/// {uri}\n
/// {query}\n
/// {headers}\n
/// {payload_hash}
///
/// Where headers is a list of headers in the following format:
/// {header}:{value}\n
///
/// The payload_hash is the sha256 hash provided in the x-scuf-content-sha256
/// header. If the x-scuf-content-sha256 header is not provided, the
/// payload_hash is an empty string. Hmac is calculated using the following key:
/// SCUF-HMAC-SHA256/{date}/{organization_id}/{access_token_id}/{secret_token}
///
/// We then verify the signature by comparing it to the hmac of the canonical
/// request.
///
/// Special Headers:
///
/// The x-scuf-content-sha256 is the sha256 hash of the request body.
/// This header is optional but can be used to sign the request body.
///
/// The date or x-scuf-date header is optional but can be used to verify the
/// request date. This can be used to prevent replay attacks.
pub async fn auth_middleware_impl<G: ApiGlobal>(
	mut req: http::Request<hyper::Body>,
) -> tonic::Result<http::Request<hyper::Body>> {
	let mut global = req
		.extensions()
		.get::<Arc<G>>()
		.ok_or_else(|| tonic::Status::internal("global state missing"))?;

	let (organization_id, access_token_id, date, signed_headers, signature) = req
		.headers()
		.get(hyper::header::AUTHORIZATION)
		.ok_or_else(|| Status::unauthenticated("no authorization header"))?
		.to_str()
		.ok()
		.and_then(|s| s.strip_prefix("SCUF-HMAC-SHA256 "))
		.and_then(|s| {
			let split = s.splitn(3, ',');
			let mut credential = None;
			let mut signed_headers = None;
			let mut signature = None;

			for part in split {
				let mut part = part.splitn(2, '=');

				match part.next()? {
					"Credential" if credential.is_none() => credential = Some(part.next()?),
					"SignedHeaders" if signed_headers.is_none() => signed_headers = Some(part.next()?),
					"Signature" if signature.is_none() => signature = Some(part.next()?),
					_ => return None,
				}
			}

			let mut credential = credential?.splitn(3, '/');
			let signed_headers = signed_headers?.split(';');
			let signature = signature?;
			if signature.len() != 64 {
				return None;
			}

			let organization_id = credential.next()?;
			let access_token_id = credential.next()?;

			// date is in the format YYYYMMDD
			let date = credential.next()?.parse::<u32>().ok()?;

			let organization_id = Ulid::from_str(organization_id).ok()?;
			let access_token_id = Ulid::from_str(access_token_id).ok()?;

			let signature = hex::decode(signature).ok()?;

			Some((organization_id, access_token_id, date, signed_headers, signature))
		})
		.ok_or_else(|| Status::unauthenticated("invalid authorization header"))?;

	if date != {
		let now = chrono::Utc::now();

		now.year() as u32 * 10000 + now.month() * 100 + now.day()
	} {
		return Err(Status::unauthenticated("invalid authorization header"));
	}

	let headers_to_hash = signed_headers
		.into_iter()
		.map(|header| {
			if !header.is_ascii()
				|| header.len() > 100
				|| header
					.chars()
					.any(|c| !(c.is_ascii_alphanumeric() && c.is_ascii_lowercase()) && c != '-' && c != '_')
			{
				return Err(());
			}

			let value = req.headers().get(header).ok_or(())?.to_str().map_err(|_| ())?.trim();

			Ok((header, value))
		})
		.collect::<Result<Vec<_>, _>>()
		.map_err(|_| Status::unauthenticated("invalid authorization header"))?
		.into_iter()
		.fold(String::new(), |mut item, (header, value)| {
			item.push_str(header);
			item.push(':');
			item.push_str(value);
			item.push('\n');
			item
		});

	if let Some(date) = req
		.headers()
		.get(X_SCUF_DATE)
		.or_else(|| req.headers().get(hyper::header::DATE))
	{
		// '20130524T000000Z' or 'Fri, 24 May 2013 00:00:00 GMT'
		let date = date
			.to_str()
			.ok()
			.and_then(|s| {
				chrono::NaiveDateTime::parse_from_str(s, X_SCUF_DATE_FMT)
					.or_else(|_| DateTime::parse_from_rfc2822(s).map(|d| d.naive_utc()))
					.ok()
			})
			.map(|d| d.timestamp())
			.ok_or_else(|| Status::unauthenticated("invalid x-scuf-date header"))?;

		let now = chrono::Utc::now().timestamp();

		// If the date header is older than 60 seconds, reject it
		if (now - date).abs() > 60 {
			return Err(Status::unauthenticated("invalid x-scuf-date header"));
		}
	}

	let payload_hash = if let Some(content_sha_256) = req.headers().get(X_SCUF_CONTENT_SHA256) {
		let hash = content_sha_256
			.to_str()
			.ok()
			.and_then(|s| match s {
				s if s.len() == 64 => Some(hex::decode(s).ok()?),
				_ => None,
			})
			.ok_or_else(|| Status::unauthenticated("invalid x-scuf-content-sha256 header"))?;

		let body = req.body_mut();
		let data = match body.data().await {
			Some(Ok(data)) => data,
			None => Bytes::new(),
			Some(Err(err)) => {
				tracing::error!(err = %err, "failed to read body");
				return Err(Status::cancelled("failed to read body"));
			}
		};

		let expected_hash = sha2::Sha256::digest(&data);

		if expected_hash.as_slice() != hash {
			return Err(Status::unauthenticated("invalid x-scuf-content-sha256 header"));
		}

		// We have to reset global here because we might have been relocated when
		// reading the body
		global = req
			.extensions_mut()
			.get_mut::<Arc<G>>()
			.ok_or_else(|| Status::internal("global state missing"))?;

		hash
	} else {
		sha2::Sha256::digest(b"").to_vec()
	}
	.encode_hex::<String>();

	let access_token = global
		.access_token_loader()
		.load((organization_id, access_token_id))
		.await
		.map_err(|()| Status::internal("failed to load access token"))?
		.ok_or_else(|| Status::unauthenticated("invalid access token"))?;

	let signing_key = format!(
		"SCUF-HMAC-SHA256/{date}/{organization_id}/{access_token_id}/{}",
		access_token.secret_token.0
	);

	let mut hmac = hmac::Hmac::<sha2::Sha256>::new_from_slice(signing_key.as_bytes())
		.map_err(|_| Status::internal("failed to create hmac"))?;

	let canonical_request = format!(
		"{method}\n{uri}\n{query}\n{headers}\n{payload_hash}",
		method = req.method(),
		uri = req.uri().path(),
		query = req.uri().query().unwrap_or(""),
		headers = headers_to_hash,
		payload_hash = payload_hash,
	);

	hmac.update(canonical_request.as_bytes());

	hmac.verify_slice(&signature)
		.map_err(|_| Status::unauthenticated("invalid authorization header"))?;

	req.extensions_mut().insert(access_token);

	Ok(req)
}

pub fn validate_request<'a, T>(
	req: &'a tonic::Request<T>,
	required_scope: &RequiredScope,
) -> Result<&'a AccessToken, Status> {
	let access_token = req
		.extensions()
		.get::<AccessToken>()
		.ok_or_else(|| Status::internal("access token missing"))?;

	access_token.has_scope(required_scope)?;

	Ok(access_token)
}
