use async_graphql::{SimpleObject, Union};

use super::session::Session;
use super::ulid::GqlUlid;
use super::user::User;
use crate::global::ApiGlobal;

#[derive(Clone, SimpleObject)]
pub struct TotpSecret {
	/// Base64 encoded totp qr code.
	pub qr_code: String,
	/// List of backup codes.
	pub backup_codes: Vec<String>,
}

#[derive(Clone, SimpleObject)]
pub struct TwoFaRequest {
	pub id: GqlUlid,
}

#[derive(Union)]
#[graphql(concrete(name = "LoginResponse", params(Session)))]
#[graphql(concrete(name = "ChangePasswordResponse", params("User<G>"), bounds("G: ApiGlobal")))]
pub enum TwoFaResponse<S: Send + Sync + async_graphql::ObjectType> {
	TwoFaRequest(TwoFaRequest),
	Success(S),
}
