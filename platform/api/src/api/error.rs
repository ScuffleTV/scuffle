use utils::http::RouteError;

use super::auth::AuthError;
use crate::turnstile::TurnstileError;

pub type Result<T, E = RouteError<ApiError>> = std::result::Result<T, E>;

#[derive(thiserror::Error, Debug)]
pub enum ApiError {
	#[error("failed to upgrade ws connection: {0}")]
	WsUpgrade(#[from] hyper_tungstenite::tungstenite::error::ProtocolError),
	#[error("failed to parse http body: {0}")]
	ParseHttpBody(#[from] hyper::Error),
	#[error("failed to parse gql request: {0}")]
	ParseGql(#[from] async_graphql::ParseRequestError),
	#[error("failed to authenticate request: {0}")]
	Auth(AuthError),
	#[error("failed to query turnstile: {0}")]
	Turnstile(#[from] TurnstileError),
	#[error("failed to query database: {0}")]
	Database(#[from] utils::database::deadpool_postgres::PoolError),
}

impl From<utils::database::tokio_postgres::Error> for ApiError {
	fn from(value: utils::database::tokio_postgres::Error) -> Self {
		Self::Database(value.into())
	}
}
