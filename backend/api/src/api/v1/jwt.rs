use std::sync::Arc;

use chrono::{DateTime, TimeZone, Utc};
use common::types::session;
use hmac::{Hmac, Mac};
use jwt::{Claims, Header, RegisteredClaims, SignWithKey, Token, VerifyWithKey};
use sha2::Sha256;

use crate::global::GlobalState;

pub struct JwtState {
    pub user_id: i64,
    pub session_id: i64,
    pub expiration: Option<DateTime<Utc>>,
    pub issued_at: DateTime<Utc>,
    pub not_before: Option<DateTime<Utc>>,
    pub audience: Option<String>,
}

impl JwtState {
    pub fn serialize(&self, global: &Arc<GlobalState>) -> Option<String> {
        let key = Hmac::<Sha256>::new_from_slice(global.config.jwt_secret.as_bytes()).ok()?;
        let claims = Claims::new(RegisteredClaims {
            issued_at: Some(self.issued_at.timestamp() as u64),
            expiration: self.expiration.map(|x| x.timestamp() as u64),
            issuer: Some(global.config.jwt_issuer.to_string()),
            json_web_token_id: Some(self.session_id.to_string()),
            subject: Some(self.user_id.to_string()),
            not_before: self.not_before.map(|x| x.timestamp() as u64),
            audience: self.audience.clone(),
        });

        claims.sign_with_key(&key).ok()
    }

    pub fn verify(global: &Arc<GlobalState>, token: &str) -> Option<Self> {
        let key = Hmac::<Sha256>::new_from_slice(global.config.jwt_secret.as_bytes()).ok()?;
        let token: Token<Header, Claims, _> = token.verify_with_key(&key).ok()?;

        let claims = token.claims();

        if claims.registered.issuer.clone()? != global.config.jwt_issuer {
            return None;
        }

        let iat = Utc
            .timestamp_opt(claims.registered.issued_at? as i64, 0)
            .single()?;
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

        let user_id = claims.registered.subject.clone()?.parse::<i64>().ok()?;

        let session_id = claims
            .registered
            .json_web_token_id
            .clone()?
            .parse::<i64>()
            .ok()?;
        let audience = claims.registered.audience.clone();

        Some(JwtState {
            user_id,
            session_id,
            expiration: exp,
            issued_at: iat,
            not_before: nbf,
            audience,
        })
    }
}

impl From<session::Model> for JwtState {
    fn from(session: session::Model) -> Self {
        JwtState {
            user_id: session.user_id,
            session_id: session.id,
            expiration: Some(session.expires_at),
            issued_at: session.created_at,
            not_before: None,
            audience: None,
        }
    }
}
