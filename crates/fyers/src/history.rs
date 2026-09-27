use std::format;

use crate::client::FyersClient;
use crate::model::Candle;
use anyhow::{bail, Result};
use serde::Deserialize;

#[derive(Deserialize)]
struct HistoryResponse {
    s: String,
    message: Option<String>,
    candles: Option<Vec<(i64, f64, f64, f64, f64, i64)>>,
}

impl FyersClient {
    pub async fn get_history(
        &self,
        symbol: &str,
        resolution: &str,
        from: &str,
        to: &str,
    ) -> Result<Vec<Candle>> {
        let token = self
            .access_token
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Access token not set. Call generate_token first."))?;

        let url = "https://api-t1.fyers.in/data/history".to_string();

        let query = [
            ("symbol", symbol),
            ("resolution", resolution),
            ("date_format", "1"),
            ("range_from", from),
            ("range_to", to),
            ("cont_flag", "1"),
        ];

        let auth_header = format!("{}:{}", self.app_id, token);

        let resp: HistoryResponse = self
            .http
            .get(&url)
            .header("Authorization", auth_header)
            .query(&query)
            .send()
            .await?
            .json()
            .await?;

        if resp.s == "ok" {
            let candles = resp
                .candles
                .unwrap_or_default()
                .into_iter()
                .map(|c| Candle {
                    date: c.0,
                    open: c.1,
                    high: c.2,
                    low: c.3,
                    close: c.4,
                    volume: c.5,
                })
                .collect();

            Ok(candles)
        } else {
            bail!(
                "Failed to fetch history: {}",
                resp.message.unwrap_or_default()
            );
        }
    }
}
