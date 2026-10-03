pub fn rsi(prices: &[f64], period: usize) -> Vec<f64> {
    let mut result = vec![f64::NAN; prices.len()];

    if prices.len() <= period || period == 0 {
        return result;
    }

    let mut gain_sum = 0.0;
    let mut loss_sum = 0.0;

    // 1. Calculate standard moving average of gains/losses for the first period
    for i in 1..=period {
        let diff = prices[i] - prices[i - 1];
        if diff > 0.0 {
            gain_sum += diff;
        } else {
            loss_sum += diff.abs();
        }
    }

    let mut avg_gain = gain_sum / period as f64;
    let mut avg_loss = loss_sum / period as f64;

    result[period] = if avg_loss == 0.0 {
        100.0
    } else {
        100.0 - (100.0 / (1.0 + (avg_gain / avg_loss)))
    };

    // 2. Apply Wilder's Smoothing for all subsequent periods
    for i in (period + 1)..prices.len() {
        let diff = prices[i] - prices[i - 1];
        let (gain, loss) = if diff > 0.0 {
            (diff, 0.0)
        } else {
            (0.0, diff.abs())
        };

        avg_gain = ((avg_gain * (period - 1) as f64) + gain) / period as f64;
        avg_loss = ((avg_loss * (period - 1) as f64) + loss) / period as f64;

        result[i] = if avg_loss == 0.0 {
            100.0
        } else {
            100.0 - (100.0 / (1.0 + (avg_gain / avg_loss)))
        };
    }

    result
}
