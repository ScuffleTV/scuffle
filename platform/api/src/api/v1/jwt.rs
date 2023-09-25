use std::sync::Arc;

use chrono::{DateTime, TimeZone, Utc};
use hmac::{Hmac, Mac};
use jwt::{Claims, Header, RegisteredClaims, SignWithKey, Token, VerifyWithKey};
use sha2::Sha256;
use ulid::Ulid;

use crate::{database::Session, global::GlobalState};

pub struct JwtState {
    pub user_id: Ulid,
    pub session_id: Ulid,
    pub expiration: Option<DateTime<Utc>>,
    pub issued_at: DateTime<Utc>,
    pub not_before: Option<DateTime<Utc>>,
    pub audience: Option<String>,
}

impl JwtState {
    pub fn serialize(&self, global: &Arc<GlobalState>) -> Option<String> {
        let key = Hmac::<Sha256>::new_from_slice(global.config.jwt.secret.as_bytes()).ok()?;
        let claims = Claims::new(RegisteredClaims {
            issued_at: Some(self.issued_at.timestamp() as u64),
            expiration: self.expiration.map(|x| x.timestamp() as u64),
            issuer: Some(global.config.jwt.issuer.to_string()),
            json_web_token_id: Some(self.session_id.to_string()),
            subject: Some(self.user_id.to_string()),
            not_before: self.not_before.map(|x| x.timestamp() as u64),
            audience: self.audience.clone(),
        });

        claims.sign_with_key(&key).ok()
    }

    pub fn verify(global: &Arc<GlobalState>, token: &str) -> Option<Self> {
        let key = Hmac::<Sha256>::new_from_slice(global.config.jwt.secret.as_bytes()).ok()?;
        let token: Token<Header, Claims, _> = token.verify_with_key(&key).ok()?;

        let claims = token.claims();

        if claims.registered.issuer.clone()? != global.config.jwt.issuer {
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

        let user_id = claims.registered.subject.clone()?.parse::<Ulid>().ok()?;

        let session_id = claims
            .registered
            .json_web_token_id
            .clone()?
            .parse::<Ulid>()
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

impl From<Session> for JwtState {
    fn from(session: Session) -> Self {
        JwtState {
            user_id: session.user_id.0,
            session_id: session.id.0,
            expiration: Some(session.expires_at),
            issued_at: Ulid::from(session.id).datetime().into(),
            not_before: None,
            audience: None,
        }
    }
}
