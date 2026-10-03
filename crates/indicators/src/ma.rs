pub fn sma(prices: &[f64], period: usize) -> Vec<f64> {
    let mut result = vec![f64::NAN; prices.len()];

    if prices.len() < period || period == 0 {
        return result;
    }

    // Calculate the initial sum for the first window
    let mut sum: f64 = prices.iter().take(period).sum();
    result[period - 1] = sum / period as f64;

    // Rolling sum for the rest
    for i in period..prices.len() {
        sum += prices[i] - prices[i - period];
        result[i] = sum / period as f64;
    }

    result
}
