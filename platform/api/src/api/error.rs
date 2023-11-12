use common::http::RouteError;

pub type Result<T, E = RouteError<ApiError>> = std::result::Result<T, E>;

#[derive(thiserror::Error, Debug)]
pub enum ApiError {
    #[error("failed to upgrade ws connection: {0}")]
    WsUpgrade(#[from] hyper_tungstenite::tungstenite::error::ProtocolError),
    #[error("failed to parse http body: {0}")]
    ParseHttpBody(#[from] hyper::Error),
    #[error("failed to parse gql request: {0}")]
    ParseGql(#[from] async_graphql::ParseRequestError),
}
