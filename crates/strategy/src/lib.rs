use fyers::model::Candle;
use indicators::bb::bollinger_bands;
use indicators::kc::keltner_channels;
use indicators::ma::sma;
use indicators::rsi::rsi;
use indicators::vwap::session_vwap;

pub enum Signal {
    Buy,
    Sell,
    Hold,
}

pub trait Strategy {
    fn check_signal(&self, candles: &[Candle], position: Option<f64>) -> Signal;
}

pub struct RsiReversionStrategy;
impl Strategy for RsiReversionStrategy {
    fn check_signal(&self, candles: &[Candle], position: Option<f64>) -> Signal {
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
    fn check_signal(&self, candles: &[Candle], position: Option<f64>) -> Signal {
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
    fn check_signal(&self, candles: &[Candle], position: Option<f64>) -> Signal {
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

pub struct VwapPullbackStrategy;

impl Strategy for VwapPullbackStrategy {
    fn check_signal(&self, candles: &[Candle], position: Option<f64>) -> Signal {
        let vwap_vals = session_vwap(candles);
        let current_vwap = vwap_vals.last().unwrap_or(&0.0);
        let current_close = candles.last().map(|c| c.close).unwrap_or(0.0);
        let current_low = candles.last().map(|c| c.low).unwrap_or(0.0);

        if *current_vwap == 0.0 {
            return Signal::Hold;
        }

        if current_low <= *current_vwap && current_close > *current_vwap {
            Signal::Buy
        } else if current_close < *current_vwap {
            Signal::Sell
        } else {
            Signal::Hold
        }
    }
}

pub struct VolatilitySqueezeStrategy;

impl Strategy for VolatilitySqueezeStrategy {
    fn check_signal(&self, candles: &[Candle], position: Option<f64>) -> Signal {
        let closes: Vec<f64> = candles.iter().map(|c| c.close).collect();
        let highs: Vec<f64> = candles.iter().map(|c| c.high).collect();
        let lows: Vec<f64> = candles.iter().map(|c| c.low).collect();

        let bb = bollinger_bands(&closes, 20, 2.0);
        let kc = keltner_channels(&closes, &highs, &lows, 20, 1.5);

        let macro_trend = sma(&closes, 200);

        let bb_upper = bb.upper.last().unwrap_or(&0.0);
        let bb_lower = bb.lower.last().unwrap_or(&0.0);
        let kc_upper = kc.upper.last().unwrap_or(&0.0);
        let kc_lower = kc.lower.last().unwrap_or(&0.0);
        let current_close = closes.last().unwrap_or(&0.0);
        let current_trend = macro_trend.last().unwrap_or(&0.0);

        if bb_upper.is_nan() || kc_upper.is_nan() || current_trend.is_nan() {
            return Signal::Hold;
        }

        if let Some(entry_price) = position {
            let stop_loss = entry_price * (1.0 - 0.003);
            let take_profit = entry_price * (1.0 + 0.009);

            if *current_close <= stop_loss {
                return Signal::Sell;
            }
            if *current_close >= take_profit {
                return Signal::Sell;
            }
            return Signal::Hold;
        }

        let is_squeezed = bb_upper < kc_upper && bb_lower > kc_lower;

        if !is_squeezed && current_close > bb_upper && current_close > current_trend {
            Signal::Buy
        } else {
            Signal::Hold
        }
    }
}
