use fyers::model::Candle;
use indicators::bb::bollinger_bands;
use indicators::ma::sma;
use indicators::rsi::rsi;
use serde::{Deserialize, Serialize};

pub enum Signal {
    Buy,
    Sell,
    Hold,
}

pub trait Strategy {
    fn check_signal(&self, candles: &[Candle]) -> Signal;
}

pub struct RsiReversionStrategy;
impl Strategy for RsiReversionStrategy {
    fn check_signal(&self, candles: &[Candle]) -> Signal {
        let closes: Vec<f64> = candles.iter().map(|c| c.close).collect();
        let rsi_vals = rsi(&closes, 14);
        let current_rsi = rsi_vals.last().unwrap_or(&50.0);

        if *current_rsi < 45.0 {
            Signal::Buy
        } else if *current_rsi > 55.0 {
            Signal::Sell
        } else {
            Signal::Hold
        }
    }
}

pub struct TrendFilteredRsiStrategy;
impl Strategy for TrendFilteredRsiStrategy {
    fn check_signal(&self, candles: &[Candle]) -> Signal {
        let closes: Vec<f64> = candles.iter().map(|c| c.close).collect();
        let rsi_vals = rsi(&closes, 14);
        let sma_vals = sma(&closes, 50);

        let current_rsi = rsi_vals.last().unwrap_or(&50.0);
        let current_sma = sma_vals.last().unwrap_or(&0.0);
        let current_price = closes.last().unwrap_or(&0.0);

        if *current_rsi < 40.0 && current_price > current_sma {
            Signal::Buy
        } else if *current_rsi > 60.0 {
            Signal::Sell
        } else {
            Signal::Hold
        }
    }
}
pub struct BollingerReversionStrategy;

impl Strategy for BollingerReversionStrategy {
    fn check_signal(&self, candles: &[Candle]) -> Signal {
        let closes: Vec<f64> = candles.iter().map(|c| c.close).collect();

        let bands = bollinger_bands(&closes, 20, 2.0);

        let current_close = closes.last().unwrap_or(&0.0);
        let current_lower = bands.lower.last().unwrap_or(&0.0);
        let current_middle = bands.middle.last().unwrap_or(&0.0);

        if current_lower.is_nan() {
            return Signal::Hold;
        }

        if current_close < current_lower {
            Signal::Buy
        } else if current_close >= current_middle {
            Signal::Sell
        } else {
            Signal::Hold
        }
    }
}

#[derive(Serialize)]
struct PredictPayload<'a> {
    symbol: &'a str,
    candles: &'a [Candle],
}

#[derive(Deserialize)]
struct PredictReply {
    signal: String,
    predicted_return_pct: f64,
}

pub struct KronosStrategy {
    pub endpoint: String,
}

impl Default for KronosStrategy {
    fn default() -> Self {
        Self {
            endpoint: "http://127.0.0.1:8000/predict".to_string(),
        }
    }
}

impl Strategy for KronosStrategy {
    fn check_signal(&self, candles: &[Candle]) -> Signal {
        if candles.len() < 30 {
            return Signal::Hold;
        }

        // Send the recent context window (e.g., last 200 candles) to Python
        let window_size = 200.min(candles.len());
        let slice = &candles[candles.len() - window_size..];

        let payload = PredictPayload {
            symbol: "NSE:STOCK",
            candles: slice,
        };

        let response = ureq::post(&self.endpoint)
            .send_json(&payload);

        match response {
            Ok(res) => {
                if let Ok(reply) = res.into_json::<PredictReply>() {
                    match reply.signal.as_str() {
                        "Buy" => Signal::Buy,
                        "Sell" => Signal::Sell,
                        _ => Signal::Hold,
                    }
                } else {
                    Signal::Hold
                }
            }
            Err(e) => {
                eprintln!("Kronos bridge error: {e}");
                Signal::Hold
            }
        }
    }
}
