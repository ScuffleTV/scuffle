use std::convert::Infallible;
use std::sync::Arc;

use hyper::body::HttpBody;
use hyper::{Body, Request, Response, StatusCode};
use routerify::prelude::RequestExt;
use routerify::Router;

use crate::global::GlobalState;

#[derive(serde::Deserialize)]
struct FetchUserRequest {
    id: Vec<i64>,
}

#[derive(serde::Serialize)]
pub struct ErrorResponse {
    code: i32,
    message: String,
}

impl ErrorResponse {
    pub fn new(code: i32, message: String) -> Self {
        Self { code, message }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("failed to serialize error response")
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
struct FetchUserResponse {
    data: Vec<User>,
    count: usize,
}

impl FetchUserResponse {
    pub fn new(data: Vec<User>) -> Self {
        let count = data.len();
        Self { data, count }
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).expect("failed to serialize fetch user response")
    }
}

#[derive(serde::Serialize, serde::Deserialize)]
struct User {
    id: String,
    username: String,
    created_at: String,
}

async fn fetch_users(mut req: Request<Body>) -> Result<Response<Body>, Infallible> {
    // Get the request body as json and deserialize it into a GetUserRequest
    let Some(Ok(data)) = req.body_mut().data().await else {
        return Ok(Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .header("content-type", "application/json")
            .body(ErrorResponse::new(400, "body has no data".to_string()).to_json().into())
            .expect("failed to build fetch user response")); 
    };

    let Ok(request) = serde_json::from_slice::<FetchUserRequest>(&data) else {
        return Ok(Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .header("content-type", "application/json")
            .body(ErrorResponse::new(400, "body is not valid json".to_string()).to_json().into())
            .expect("failed to build fetch user response"));
    };

    if request.id.is_empty() {
        return Ok(Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .header("content-type", "application/json")
            .body(
                ErrorResponse::new(400, "request has no ids".to_string())
                    .to_json()
                    .into(),
            )
            .expect("failed to build fetch user response"));
    }

    if request.id.len() > 100 {
        return Ok(Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .header("content-type", "application/json")
            .body(
                ErrorResponse::new(
                    400,
                    "you cannot request more than 100 users in a single request".to_string(),
                )
                .to_json()
                .into(),
            )
            .expect("failed to build fetch user response"));
    }

    // Do something with the request

    let global = req.data::<Arc<GlobalState>>().unwrap();

    let result = match sqlx::query!(
        r#"
        SELECT id, username, created_at
        FROM users
        WHERE id = ANY($1)
        "#,
        &request.id
    )
    .map(|row| User {
        id: row.id.to_string(),
        username: row.username,
        created_at: row.created_at.to_string(),
    })
    .fetch_all(&global.db)
    .await
    {
        Ok(result) => result,
        Err(err) => {
            tracing::error!("failed to fetch users: {}", err);
            return Ok(Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .header("content-type", "application/json")
                .body(
                    ErrorResponse::new(500, "failed to fetch users".to_string())
                        .to_json()
                        .into(),
                )
                .expect("failed to build fetch user response"));
        }
    };

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header("content-type", "application/json")
        .body(FetchUserResponse::new(result).to_json().into())
        .expect("failed to build fetch user response"))
}

pub fn routes() -> Router<Body, Infallible> {
    Router::builder().post("/", fetch_users).build().unwrap()
}

#[cfg(test)]
mod tests;
