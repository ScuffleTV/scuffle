use serde::de::Error;
use serde_json::json;

use super::GlobalState;

#[derive(Debug, thiserror::Error)]
pub enum TurnstileError {
    #[error("reqwest error: {0}")]
    Reqwest(#[from] reqwest::Error),
    #[error("json error: {0}")]
    SerdeJson(#[from] serde_json::Error),
}

impl GlobalState {
    pub async fn validate_turnstile_token(&self, token: &str) -> Result<bool, TurnstileError> {
        let client = reqwest::Client::new();

        let body = json!({
            "response": token,
            "secret": self.config.turnstile.secret_key,
        });

        let res = client
            .post(self.config.turnstile.url.as_str())
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        let body = res.json::<serde_json::Value>().await?;

        Ok(body["success"]
            .as_bool()
            .ok_or(serde_json::Error::missing_field("success"))?)
    }
}
