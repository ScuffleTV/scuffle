use std::sync::Arc;

use serde::de::Error;
use serde_json::json;

use crate::config::TurnstileConfig;
use crate::global::ApiGlobal;

#[derive(Debug, thiserror::Error)]
pub enum TurnstileError {
	#[error("reqwest error: {0}")]
	Reqwest(#[from] reqwest::Error),
	#[error("json error: {0}")]
	SerdeJson(#[from] serde_json::Error),
}

pub async fn validate_turnstile_token<G: ApiGlobal>(global: &Arc<G>, token: &str) -> Result<bool, TurnstileError> {
	let client = reqwest::Client::new();

	let config = global.config::<TurnstileConfig>();

	let body = json!({
		"response": token,
		"secret": config.secret_key,
	});

	let res = client
		.post(config.url.as_str())
		.header("Content-Type", "application/json")
		.json(&body)
		.send()
		.await?;

	let body = res.json::<serde_json::Value>().await?;

	Ok(body["success"].as_bool().ok_or(serde_json::Error::missing_field("success"))?)
}
