use fyers::model::Candle;
use indicators::bb::bollinger_bands;
use indicators::kc::keltner_channels;
use indicators::rsi::rsi;
use indicators::vwap::session_vwap;

pub enum Signal {
    Buy,
    Sell,
    Short,
    Cover,
    Hold,
}

pub trait Strategy {
    fn check_signal(&self, candles: &[Candle], _position: Option<f64>) -> Signal;
}

pub struct RsiReversionStrategy;
impl Strategy for RsiReversionStrategy {
    fn check_signal(&self, candles: &[Candle], _position: Option<f64>) -> Signal {
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
    fn check_signal(&self, candles: &[Candle], _position: Option<f64>) -> Signal {
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
    fn check_signal(&self, candles: &[Candle], _position: Option<f64>) -> Signal {
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
    fn check_signal(&self, candles: &[Candle], _position: Option<f64>) -> Signal {
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
        let atr_vals = atr(candles, 14);
        let adx_vals = adx(candles, 14);

        // Fetch RSI using your crate's existing function
        let rsi_vals = rsi(&closes, 14);

        let bb_upper = bb.upper.last().unwrap_or(&0.0);
        let bb_lower = bb.lower.last().unwrap_or(&0.0);
        let kc_upper = kc.upper.last().unwrap_or(&0.0);
        let kc_lower = kc.lower.last().unwrap_or(&0.0);

        let current_close = closes.last().unwrap_or(&0.0);
        let current_trend = macro_trend.last().unwrap_or(&0.0);
        let current_atr = atr_vals.last().unwrap_or(&0.0);
        let current_adx = adx_vals.last().unwrap_or(&0.0);

        if bb_upper.is_nan()
            || kc_upper.is_nan()
            || current_trend.is_nan()
            || current_atr.is_nan()
            || current_adx.is_nan()
        {
            return Signal::Hold;
        }

        if let Some(pos_price) = position {
            let is_long = pos_price > 0.0;
            let entry_price = pos_price.abs();

            if is_long {
                let stop_loss = entry_price - (2.0 * current_atr);
                let take_profit = entry_price + (6.0 * current_atr);
                if *current_close <= stop_loss || *current_close >= take_profit {
                    return Signal::Sell;
                }
            } else {
                let stop_loss = entry_price + (2.0 * current_atr);
                let take_profit = entry_price - (6.0 * current_atr);
                if *current_close >= stop_loss || *current_close <= take_profit {
                    return Signal::Cover;
                }
            }
            return Signal::Hold;
        }

        let is_squeezed = bb_upper < kc_upper && bb_lower > kc_lower;

        let bb_upper_prev = bb.upper.iter().rev().nth(1).unwrap_or(&0.0);
        let bb_lower_prev = bb.lower.iter().rev().nth(1).unwrap_or(&0.0);
        let kc_upper_prev = kc.upper.iter().rev().nth(1).unwrap_or(&0.0);
        let kc_lower_prev = kc.lower.iter().rev().nth(1).unwrap_or(&0.0);

        let was_squeezed_recently = bb_upper_prev < kc_upper_prev && bb_lower_prev > kc_lower_prev;

        if !is_squeezed && was_squeezed_recently && *current_adx > 20.0 {
            if current_close > bb_upper && current_close > current_trend {
                return Signal::Buy;
            } else if current_close < bb_lower && current_close < current_trend {
                return Signal::Short;
            }
        }

        Signal::Hold
    }
}

pub fn atr(candles: &[Candle], period: usize) -> Vec<f64> {
    let mut atr_vals = Vec::with_capacity(candles.len());
    if candles.is_empty() {
        return atr_vals;
    }

    let mut tr_vals = Vec::with_capacity(candles.len());
    tr_vals.push(candles[0].high - candles[0].low);

    for i in 1..candles.len() {
        let high = candles[i].high;
        let low = candles[i].low;
        let prev_close = candles[i - 1].close;

        let tr1 = high - low;
        let tr2 = (high - prev_close).abs();
        let tr3 = (low - prev_close).abs();

        tr_vals.push(tr1.max(tr2).max(tr3));
    }

    let mut current_atr = tr_vals.iter().take(period).sum::<f64>() / period as f64;
    for i in 0..period {
        atr_vals.push(if i == period - 1 { current_atr } else { 0.0 });
    }

    for i in period..candles.len() {
        current_atr = ((current_atr * (period as f64 - 1.0)) + tr_vals[i]) / period as f64;
        atr_vals.push(current_atr);
    }
    atr_vals
}

pub fn adx(candles: &[Candle], period: usize) -> Vec<f64> {
    let n = candles.len();
    let mut adx_vals = vec![0.0; n];
    if n <= period * 2 {
        return adx_vals;
    }

    let mut tr = vec![0.0; n];
    let mut plus_dm = vec![0.0; n];
    let mut minus_dm = vec![0.0; n];

    for i in 1..n {
        let h1 = candles[i].high;
        let l1 = candles[i].low;
        let h0 = candles[i - 1].high;
        let l0 = candles[i - 1].low;
        let c0 = candles[i - 1].close;

        tr[i] = (h1 - l1).max((h1 - c0).abs()).max((l1 - c0).abs());

        let up_move = h1 - h0;
        let down_move = l0 - l1;

        if up_move > down_move && up_move > 0.0 {
            plus_dm[i] = up_move;
        }
        if down_move > up_move && down_move > 0.0 {
            minus_dm[i] = down_move;
        }
    }

    let smooth = |data: &[f64]| -> Vec<f64> {
        let mut smoothed = vec![0.0; n];
        let mut sum = 0.0;
        for i in 1..=period {
            sum += data[i];
        }
        smoothed[period] = sum;
        for i in (period + 1)..n {
            smoothed[i] = smoothed[i - 1] - (smoothed[i - 1] / period as f64) + data[i];
        }
        smoothed
    };

    let smoothed_tr = smooth(&tr);
    let smoothed_plus = smooth(&plus_dm);
    let smoothed_minus = smooth(&minus_dm);

    let mut dx = vec![0.0; n];
    for i in period..n {
        if smoothed_tr[i] == 0.0 {
            continue;
        }
        let plus_di = 100.0 * (smoothed_plus[i] / smoothed_tr[i]);
        let minus_di = 100.0 * (smoothed_minus[i] / smoothed_tr[i]);
        let sum_di = plus_di + minus_di;
        if sum_di > 0.0 {
            dx[i] = 100.0 * (plus_di - minus_di).abs() / sum_di;
        }
    }

    let mut adx_sum = 0.0;
    let adx_start = period * 2;
    for i in period..adx_start {
        adx_sum += dx[i];
    }
    adx_vals[adx_start - 1] = adx_sum / period as f64;

    for i in adx_start..n {
        adx_vals[i] = ((adx_vals[i - 1] * (period as f64 - 1.0)) + dx[i]) / period as f64;
    }

    adx_vals
}

pub fn sma(prices: &[f64], period: usize) -> Vec<f64> {
    let mut sma_vals = vec![f64::NAN; prices.len()];

    if prices.len() < period || period == 0 {
        return sma_vals;
    }

    let mut sum: f64 = prices[0..period].iter().sum();
    sma_vals[period - 1] = sum / period as f64;

    for i in period..prices.len() {
        sum += prices[i] - prices[i - period];
        sma_vals[i] = sum / period as f64;
    }

    sma_vals
}
