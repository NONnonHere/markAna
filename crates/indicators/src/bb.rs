use crate::ma::sma;

pub struct BollingerBands {
    pub lower: Vec<f64>,
    pub middle: Vec<f64>,
    pub upper: Vec<f64>,
}

pub fn bollinger_bands(prices: &[f64], period: usize, multiplier: f64) -> BollingerBands {
    let mut lower = vec![f64::NAN; prices.len()];
    let mut upper = vec![f64::NAN; prices.len()];

    let middle = sma(prices, period);

    if prices.len() < period || period == 0 {
        return BollingerBands {
            lower,
            middle,
            upper,
        };
    }

    for i in (period - 1)..prices.len() {
        let window = &prices[(i + 1 - period)..=i];
        let mean = middle[i];

        let variance: f64 = window
            .iter()
            .map(|&p| {
                let diff = p - mean;
                diff * diff
            })
            .sum::<f64>()
            / period as f64;

        let std_dev = variance.sqrt();

        lower[i] = mean - (multiplier * std_dev);
        upper[i] = mean + (multiplier * std_dev);
    }

    BollingerBands {
        lower,
        middle,
        upper,
    }
}
