use anyhow::Result;
use serde::de::Error;
use serde_json::json;

use super::GlobalState;

impl GlobalState {
    pub async fn validate_turnstile_token(&self, token: &str) -> Result<bool> {
        let client = reqwest::Client::new();

        let body = json!({
            "response": token,
            "secret": self.config.turnstile_secret_key,
        });

        let res = client
            .post(self.config.turnstile_url.as_str())
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
