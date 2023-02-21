use std::time::Duration;

use common::{context::Context, logging};
use hyper::{body::Bytes, Client};

use crate::{api::run, config::AppConfig};

use super::*;

#[tokio::test]
async fn test_user_api() {
    let db = sqlx::PgPool::connect(&std::env::var("DATABASE_URL").expect("DATABASE_URL not set"))
        .await
        .expect("failed to connect to database");

    // We need to initalize logging
    logging::init("api=debug").expect("failed to initialize logging");

    let (ctx, handler) = Context::new();

    let global = Arc::new(GlobalState {
        config: AppConfig {
            bind_address: "[::]:8081".to_string(),
            database_url: "".to_string(),
            log_level: "api=debug".to_string(),
            config_file: "".to_string(),
        },
        ctx,
        db,
    });

    sqlx::query!("DELETE FROM users")
        .execute(&global.db)
        .await
        .expect("failed to delete users");
    sqlx::query!("INSERT INTO users (id, username, password_hash, email, email_verified, created_at, last_login_at) VALUES
    (1, 'admin', 'abc', 'xyz@gmail.com', true, '2021-01-01 00:00:00', '2021-01-01 00:00:00'),
    (2, 'user', 'abc2', 'xyz2@gmail.com', true, '2021-01-01 00:00:00', '2021-01-01 00:00:00'),
    (3, 'user1', 'abc3', 'xyz3@gmail.com', true, '2021-01-01 00:00:00', '2021-01-01 00:00:00'),
    (4, 'user2', 'abc4', 'xyz4@gmail.com', true, '2021-01-01 00:00:00', '2021-01-01 00:00:00'),
    (5, 'user3', 'abc5', 'xyz5@gmail.com', true, '2021-01-01 00:00:00', '2021-01-01 00:00:00'),
    (6, 'user4', 'abc6', 'xyz6@gmail.com', true, '2021-01-01 00:00:00', '2021-01-01 00:00:00'),
    (7, 'user5', 'abc7', 'xyz7@gmail.com', true, '2021-01-01 00:00:00', '2021-01-01 00:00:00'),
    (8, 'user6', 'abc8', 'xyz8@gmail.com', true, '2021-01-01 00:00:00', '2021-01-01 00:00:00'),
    (9, 'user7', 'abc9', 'xyz9@gmail.com', true, '2021-01-01 00:00:00', '2021-01-01 00:00:00'),
    (10, 'user8', 'abc10', 'xyz10@gmail.com', true, '2021-01-01 00:00:00', '2021-01-01 00:00:00'),
    (11, 'user9', 'abc11', 'xyz11@gmail.com', true, '2021-01-01 00:00:00', '2021-01-01 00:00:00');").execute(&global.db).await.expect("failed to insert users");

    let handle = tokio::spawn(run(global.clone()));

    // We need to wait for the server to start
    tokio::time::sleep(Duration::from_millis(300)).await;

    let client = Client::new();

    let resp = client
        .get("http://localhost:8081/v1/users".parse().unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::NOT_FOUND);

    let req = Request::builder()
        .method("POST")
        .uri("http://localhost:8081/v1/users")
        .body(Body::empty())
        .unwrap();

    let resp = client.request(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        resp.headers().get("content-type").unwrap(),
        "application/json"
    );

    let body = hyper::body::to_bytes(resp.into_body()).await.unwrap();
    assert_eq!(
        body,
        Bytes::from("{\"code\":400,\"message\":\"body has no data\"}")
    );

    let req = Request::builder()
        .method("POST")
        .uri("http://localhost:8081/v1/users")
        .body(Body::from("abc"))
        .unwrap();

    let resp = client.request(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        resp.headers().get("content-type").unwrap(),
        "application/json"
    );

    let body = hyper::body::to_bytes(resp.into_body()).await.unwrap();
    assert_eq!(
        body,
        Bytes::from("{\"code\":400,\"message\":\"body is not valid json\"}")
    );

    let req = Request::builder()
        .method("POST")
        .uri("http://localhost:8081/v1/users")
        .body(Body::from("{\"id\":[]}"))
        .unwrap();

    let resp = client.request(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        resp.headers().get("content-type").unwrap(),
        "application/json"
    );

    let body = hyper::body::to_bytes(resp.into_body()).await.unwrap();
    assert_eq!(
        body,
        Bytes::from("{\"code\":400,\"message\":\"request has no ids\"}")
    );

    let req = Request::builder()
        .method("POST")
        .uri("http://localhost:8081/v1/users")
        .body(Body::from("{\"id\":[1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,11,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1,1]}"))
        .unwrap();

    let resp = client.request(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    assert_eq!(
        resp.headers().get("content-type").unwrap(),
        "application/json"
    );

    let body = hyper::body::to_bytes(resp.into_body()).await.unwrap();
    assert_eq!(body.to_vec(), b"{\"code\":400,\"message\":\"you cannot request more than 100 users in a single request\"}");

    let req = Request::builder()
        .method("POST")
        .uri("http://localhost:8081/v1/users")
        .body(Body::from("{\"id\":[1,2,3,4,5,6,7,8,9,10]}"))
        .unwrap();

    let resp = client.request(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(
        resp.headers().get("content-type").unwrap(),
        "application/json"
    );

    let body = hyper::body::to_bytes(resp.into_body()).await.unwrap();

    let resp: FetchUserResponse = serde_json::from_slice(&body).unwrap();
    assert_eq!(resp.count, 10);
    assert_eq!(resp.data.len(), 10);

    // Lets try disconnect the database
    global.db.close().await;

    // Drop global so that the context is cancelled later
    drop(global);

    let req = Request::builder()
        .method("POST")
        .uri("http://localhost:8081/v1/users")
        .body(Body::from("{\"id\":[1,2,3,4,5,6,7,8,9,10]}"))
        .unwrap();

    let resp = client.request(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(
        resp.headers().get("content-type").unwrap(),
        "application/json"
    );

    let body = hyper::body::to_bytes(resp.into_body()).await.unwrap();
    assert_eq!(
        body,
        Bytes::from("{\"code\":500,\"message\":\"failed to fetch users\"}")
    );

    // The client uses Keep-Alive, so we need to drop it to release the global context
    drop(client);

    tokio::time::timeout(Duration::from_secs(1), handler.cancel())
        .await
        .expect("failed to cancel context");
    tokio::time::timeout(Duration::from_secs(1), handle)
        .await
        .expect("failed to cancel api")
        .expect("api failed")
        .expect("api failed");
}
