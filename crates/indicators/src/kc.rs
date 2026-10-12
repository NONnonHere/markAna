use crate::ma::sma;

pub struct KeltnerChannels {
    pub upper: Vec<f64>,
    pub lower: Vec<f64>,
}

pub fn keltner_channels(
    prices: &[f64],
    highs: &[f64],
    lows: &[f64],
    period: usize,
    multiplier: f64,
) -> KeltnerChannels {
    let mut upper = vec![f64::NAN; prices.len()];
    let mut lower = vec![f64::NAN; prices.len()];
    let middle = sma(prices, period);

    if prices.len() < period {
        return KeltnerChannels { upper, lower };
    }

    for i in period..prices.len() {
        // Simple ATR approximation for speed
        let mut tr_sum = 0.0;
        for j in (i + 1 - period)..=i {
            let tr1 = highs[j] - lows[j];
            let tr2 = (highs[j] - prices[j - 1]).abs();
            let tr3 = (lows[j] - prices[j - 1]).abs();
            tr_sum += tr1.max(tr2).max(tr3);
        }
        let atr = tr_sum / period as f64;

        upper[i] = middle[i] + (multiplier * atr);
        lower[i] = middle[i] - (multiplier * atr);
    }
    KeltnerChannels { upper, lower }
}
