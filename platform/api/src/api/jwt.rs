use std::sync::Arc;

use chrono::{DateTime, TimeZone, Utc};
use hmac::{Hmac, Mac};
use jwt_next::{Claims, Header, RegisteredClaims, SignWithKey, Token, VerifyWithKey};
use sha2::Sha256;
use ulid::Ulid;

use crate::config::JwtConfig;
use crate::database::Session;
use crate::global::ApiGlobal;

pub struct AuthJwtPayload {
	pub user_id: Ulid,
	pub session_id: Ulid,
	pub expiration: Option<DateTime<Utc>>,
	pub issued_at: DateTime<Utc>,
	pub not_before: Option<DateTime<Utc>>,
	pub audience: Option<String>,
}

pub trait JwtState: Sized {
	fn to_claims(&self) -> Claims;

	fn from_claims(claims: &Claims) -> Option<Self>;

	fn serialize<G: ApiGlobal>(&self, global: &Arc<G>) -> Option<String> {
		let config = global.config::<JwtConfig>();

		let key = Hmac::<Sha256>::new_from_slice(config.secret.as_bytes()).ok()?;
		let mut claims = self.to_claims();

		claims.registered.issuer = Some(config.issuer.clone());

		if claims.registered.issued_at.is_none() {
			claims.registered.issued_at = Some(chrono::Utc::now().timestamp() as u64);
		}

		claims.sign_with_key(&key).ok()
	}

	fn verify<G: ApiGlobal>(global: &Arc<G>, token: &str) -> Option<Self> {
		let config = global.config::<JwtConfig>();

		let key = Hmac::<Sha256>::new_from_slice(config.secret.as_bytes()).ok()?;
		let token: Token<Header, Claims, _> = token.verify_with_key(&key).ok()?;

		let claims = token.claims();

		if claims.registered.issuer.as_ref() != Some(&config.issuer) {
			return None;
		}

		let iat = Utc.timestamp_opt(claims.registered.issued_at? as i64, 0).single()?;
		if iat > Utc::now() {
			return None;
		}

		let nbf = claims
			.registered
			.not_before
			.and_then(|x| Utc.timestamp_opt(x as i64, 0).single());
		if let Some(nbf) = nbf {
			if nbf > Utc::now() {
				return None;
			}
		}

		let exp = claims
			.registered
			.expiration
			.and_then(|x| Utc.timestamp_opt(x as i64, 0).single());
		if let Some(exp) = exp {
			if exp < Utc::now() {
				return None;
			}
		}

		Self::from_claims(claims)
	}
}

impl JwtState for AuthJwtPayload {
	fn to_claims(&self) -> Claims {
		Claims {
			registered: RegisteredClaims {
				issuer: None,
				subject: Some(self.user_id.to_string()),
				audience: self.audience.clone(),
				expiration: self.expiration.map(|x| x.timestamp() as u64),
				not_before: self.not_before.map(|x| x.timestamp() as u64),
				issued_at: Some(self.issued_at.timestamp() as u64),
				json_web_token_id: Some(self.session_id.to_string()),
			},
			private: Default::default(),
		}
	}

	fn from_claims(claims: &Claims) -> Option<Self> {
		Some(Self {
			audience: claims.registered.audience.clone(),
			expiration: claims
				.registered
				.expiration
				.and_then(|x| Utc.timestamp_opt(x as i64, 0).single()),
			issued_at: Utc.timestamp_opt(claims.registered.issued_at? as i64, 0).single()?,
			not_before: claims
				.registered
				.not_before
				.and_then(|x| Utc.timestamp_opt(x as i64, 0).single()),
			session_id: claims
				.registered
				.json_web_token_id
				.as_ref()
				.and_then(|x| Ulid::from_string(x).ok())?,
			user_id: claims.registered.subject.as_ref().and_then(|x| Ulid::from_string(x).ok())?,
		})
	}
}

impl From<Session> for AuthJwtPayload {
	fn from(session: Session) -> Self {
		AuthJwtPayload {
			user_id: session.user_id.0,
			session_id: session.id.0,
			expiration: Some(session.expires_at),
			issued_at: Ulid::from(session.id).datetime().into(),
			not_before: None,
			audience: None,
		}
	}
}
