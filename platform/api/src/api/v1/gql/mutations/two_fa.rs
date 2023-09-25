use crate::{
    api::v1::gql::{
        error::{GqlError, Result, ResultExt},
        ext::ContextExt,
        models::two_fa::TotpSecret,
    },
    database,
};
use async_graphql::{Context, Object};

#[derive(Default, Clone)]
pub struct TwoFaMutation;

#[Object]
impl TwoFaMutation {
    /// Enable two factor authentication for the currently authenticated user.
    async fn enable<'ctx>(&self, ctx: &Context<'_>) -> Result<TotpSecret> {
        let global = ctx.get_global();
        let request_context = ctx.get_req_context();

        let auth = request_context
            .auth()
            .await
            .ok_or(GqlError::Unauthorized.with_message("You need to be logged in"))?;

        // Check if already enabled.
        let user: database::User = global
            .user_by_id_loader
            .load(auth.session.user_id.0)
            .await
            .ok()
            .map_err_gql("failed to fetch user")?
            .ok_or(GqlError::NotFound.with_message("user not found"))?;

        todo!("check totp secret is set on user struct");

        // Generate new secret.
        let secret = totp_rs::Secret::generate_secret()
            .to_bytes()
            .map_err(|_| GqlError::InternalServerError.with_message("failed generate secret"))?;
        let mut rfc = totp_rs::Rfc6238::with_defaults(secret.clone())
            .map_err(|_| GqlError::InternalServerError.with_message("failed generate secret"))?;
        rfc.issuer("Scuffle".to_string());
        rfc.account_name(user.username);

        let totp = totp_rs::TOTP::from_rfc6238(rfc).unwrap();

        // Save secret to database.
        sqlx::query("UPDATE users SET totp_secret = $1 WHERE id = $2 AND totp_secret IS NULL")
            .bind(secret)
            .bind(auth.session.user_id)
            .execute(global.db.as_ref())
            .await
            .map_err_gql("failed to update user")?;

        let qr_code = totp
            .get_qr_base64()
            .map_err(|_| GqlError::InternalServerError.with_message("failed generate qr code"))?;

        Ok(TotpSecret { qr_code })
    }
}
