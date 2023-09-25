use crate::{
    api::{
        middleware::auth::AuthError,
        v1::gql::{
            error::{GqlError, Result, ResultExt},
            ext::ContextExt,
            models::{two_fa::TotpSecret, user::User},
        },
    },
    database,
};
use async_graphql::{Context, Object};
use rand::{rngs::OsRng, RngCore};

#[derive(Default, Clone)]
pub struct TwoFaMutation;

#[Object]
impl TwoFaMutation {
    /// Generate a new TOTP secret for the currently authenticated user.
    async fn generate_totp<'ctx>(&self, ctx: &Context<'_>) -> Result<TotpSecret> {
        let global = ctx.get_global();
        let request_context = ctx.get_req_context();

        let auth = request_context
            .auth()
            .await?
            .map_err_gql(GqlError::Auth(AuthError::NotLoggedIn))?;

        let user: database::User = global
            .user_by_id_loader
            .load(auth.session.user_id.0)
            .await
            .map_err_gql("failed to fetch user")?
            .map_err_gql(GqlError::NotFound("user"))?;

        // Check if already enabled.
        if user.totp_enabled {
            return Err(GqlError::InvalidInput {
                fields: vec![],
                message: "two factor authentication is already enabled",
            }
            .into());
        }

        // Generate new secret.
        let mut secret = [0u8; 20];
        OsRng.fill_bytes(&mut secret);
        let secret = secret.to_vec();
        let mut rfc = totp_rs::Rfc6238::with_defaults(secret.clone())
            .map_err_gql("failed generate secret")?;
        rfc.issuer("Scuffle".to_string());
        rfc.account_name(user.username);

        let totp = totp_rs::TOTP::from_rfc6238(rfc).map_err_gql("failed initilize totp")?;

        // Generate backup codes.
        let mut backup_codes: Vec<i32> = Vec::with_capacity(12);
        for _ in 0..12 {
            backup_codes.push(OsRng.next_u32() as i32);
        }

        let hex_backup_codes = backup_codes.iter().map(|c| format!("{:08x}", c)).collect();

        // Save secret and backup codes to database.
        sqlx::query("UPDATE users SET totp_secret = $1, two_fa_backup_codes = $2, updated_at = NOW() WHERE id = $3")
            .bind(secret)
            .bind(backup_codes)
            .bind(auth.session.user_id)
            .execute(global.db.as_ref())
            .await?;

        let qr_code = totp
            .get_qr_base64()
            .map_err_gql("failed generate qr code")?;

        Ok(TotpSecret {
            qr_code,
            backup_codes: hex_backup_codes,
        })
    }

    /// Enable TOTP for the currently authenticated user.
    async fn enable_totp<'ctx>(&self, ctx: &Context<'_>, code: String) -> Result<User> {
        let global = ctx.get_global();
        let request_context = ctx.get_req_context();

        let auth = request_context
            .auth()
            .await?
            .map_err_gql(GqlError::Auth(AuthError::NotLoggedIn))?;

        let user: database::User = global
            .user_by_id_loader
            .load(auth.session.user_id.0)
            .await
            .map_err_gql("failed to fetch user")?
            .map_err_gql(GqlError::NotFound("user"))?;

        // Check if already enabled.
        if user.totp_enabled {
            return Err(GqlError::InvalidInput {
                fields: vec![],
                message: "two factor authentication is already enabled",
            }
            .into());
        }

        // Check if code is valid.
        if !user.verify_totp_code(&code, false)? {
            return Err(GqlError::InvalidInput {
                fields: vec!["code"],
                message: "invalid code",
            }
            .into());
        }

        // Enable 2fa
        let user: database::User = sqlx::query_as(
            "UPDATE users SET totp_enabled = true, updated_at = NOW() WHERE id = $1 RETURNING *",
        )
        .bind(auth.session.user_id)
        .fetch_one(global.db.as_ref())
        .await?;

        // TODO: Log out all other sessions?

        Ok(user.into())
    }

    /// Disable TOTP for the currently authenticated user.
    async fn disable_totp<'ctx>(&self, ctx: &Context<'ctx>, password: String) -> Result<User> {
        let global = ctx.get_global();
        let request_context = ctx.get_req_context();

        let auth = request_context
            .auth()
            .await?
            .map_err_gql(GqlError::Auth(AuthError::NotLoggedIn))?;

        let user: database::User = global
            .user_by_id_loader
            .load(auth.session.user_id.0)
            .await
            .map_err_gql("failed to fetch user")?
            .map_err_gql(GqlError::NotFound("user"))?;

        // Check password
        if !user.verify_password(&password) {
            return Err(GqlError::InvalidInput {
                fields: vec!["password"],
                message: "wrong password",
            }
            .into());
        }

        // Disable 2fa, remove secret and backup codes.
        let user: database::User = sqlx::query_as("UPDATE users SET totp_enabled = false, totp_secret = NULL, two_fa_backup_codes = NULL, updated_at = NOW() WHERE id = $1 RETURNING *")
            .bind(auth.session.user_id)
            .fetch_one(global.db.as_ref())
            .await?;

        Ok(user.into())
    }
}
