use chrono::{Datelike, TimeZone, Utc};
use fyers::model::Candle;

pub fn session_vwap(candles: &[Candle]) -> Vec<f64> {
    let mut vwap = vec![f64::NAN; candles.len()];
    let mut cum_vol = 0.0;
    let mut cum_vol_price = 0.0;
    let mut current_day = -1;

    for (i, c) in candles.iter().enumerate() {
        let dt = Utc.timestamp_opt(c.date, 0).unwrap();
        let day = dt.date_naive().day() as i32;

        // Reset cumulative totals at the start of a new trading day
        if day != current_day {
            cum_vol = 0.0;
            cum_vol_price = 0.0;
            current_day = day;
        }

        let typical_price = (c.high + c.low + c.close) / 3.0;
        let volume = c.volume as f64;

        cum_vol_price += typical_price * volume;
        cum_vol += volume;

        if cum_vol > 0.0 {
            vwap[i] = cum_vol_price / cum_vol;
        }
    }
    vwap
}
