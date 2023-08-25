use async_graphql::SimpleObject;

#[derive(Clone, SimpleObject)]
pub struct TotpSecret {
    /// Base64 encoded totp qr code.
    pub qr_code: String,
}
