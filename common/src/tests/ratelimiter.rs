use crate::ratelimiter::{load_rate_limiter_script, ratelimit, RateLimiterOptions};
use fred::prelude::*;
use ulid::Ulid;

#[tokio::test]
async fn test_load_ratelimiter() {
    dotenvy::dotenv().ok();

    let redis_url = std::env::var("REDIS_ADDR")
        .map(|addr| format!("redis://{addr}"))
        .unwrap_or_else(|_| {
            std::env::var("REDIS_URL").expect("REDIS_URL and REDIS_ADDR are not set")
        });

    let client = RedisClient::new(
        RedisConfig::from_url(&redis_url).expect("invalid redis url"),
        None,
        None,
        None,
    );
    client.connect();

    load_rate_limiter_script(&client)
        .await
        .expect("Failed to load ratelimiter script");
}

#[tokio::test]
async fn test_ratelimit() {
    dotenvy::dotenv().ok();

    let redis_url = std::env::var("REDIS_ADDR")
        .map(|addr| format!("redis://{addr}"))
        .unwrap_or_else(|_| {
            std::env::var("REDIS_URL").expect("REDIS_URL and REDIS_ADDR are not set")
        });

    let client = RedisClient::new(
        RedisConfig::from_url(&redis_url).expect("invalid redis url"),
        None,
        None,
        None,
    );
    client.connect();

    load_rate_limiter_script(&client)
        .await
        .expect("Failed to load ratelimiter script");

    let options = RateLimiterOptions {
        namespace: format!("{{{}}}", Ulid::new()),
        limit_key: "limit".to_string(),
        exceeded_key: "exceeded".to_string(),
        banned_key: "banned".to_string(),
        cost: 1,
        quota: 10,
        exceeded_limit: 100,
        quota_reset_seconds: 10,
        exceeded_reset_seconds: 30,
        banned_reset_seconds: 300,
    };

    for i in 0..10 {
        let response = ratelimit(&client, &options)
            .await
            .expect("Failed to rate limit");
        assert_eq!(response.remaining, 10 - i - 1);
        assert!(!response.banned);
        assert!(response.reset.is_some());
        assert!(response.can_request())
    }

    for _ in 0..100 {
        let response = ratelimit(&client, &options)
            .await
            .expect("Failed to rate limit");
        assert_eq!(response.remaining, -1);
        assert!(!response.banned);
        assert!(response.reset.is_some());
        assert!(!response.can_request())
    }

    let response = ratelimit(&client, &options)
        .await
        .expect("Failed to rate limit");
    assert_eq!(response.remaining, -1);
    assert!(response.banned);
    assert!(response.reset.is_some());
    assert!(!response.can_request())
}
