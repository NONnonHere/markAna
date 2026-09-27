use crate::client::{FyersClient, BASE_URL};
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct TokenRequest {
    grant_type: String,
    #[serde(rename = "appIdHash")]
    app_id_hash: String,
    code: String,
}

#[derive(Deserialize)]
struct TokenResponse {
    s: String,
    access_token: Option<String>,
    message: Option<String>,
}

impl FyersClient {
    pub async fn generate_token(&mut self, auth_code: &str) -> Result<()> {
        let req_body = TokenRequest {
            grant_type: "authorization_code".to_string(),
            app_id_hash: self.get_app_id_hash(),
            code: auth_code.to_string(),
        };

        let url = format!("{}/validate-authcode", BASE_URL);

        let resp: TokenResponse = self
            .http
            .post(&url)
            .json(&req_body)
            .send()
            .await?
            .json()
            .await?;

        if resp.s == "ok" {
            if let Some(token) = resp.access_token {
                self.set_token(token);
                Ok(())
            } else {
                bail!("Token missing from successful response");
            }
        } else {
            bail!("Auth failed: {}", resp.message.unwrap_or_default());
        }
    }
}
