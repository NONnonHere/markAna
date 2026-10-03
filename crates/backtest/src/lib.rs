use fyers::model::Candle;
use strategy::{Signal, Strategy};

pub struct Trade {
    pub entry_price: f64,
    pub exit_price: f64,
    pub pnl_percent: f64,
}

pub struct BacktestResult {
    pub initial_capital: f64,
    pub final_capital: f64,
    pub trades: Vec<Trade>,
}

impl BacktestResult {
    pub fn print_metrics(&self, symbol: &str) {
        let total_pnl = self.final_capital - self.initial_capital;
        let return_pct = (total_pnl / self.initial_capital) * 100.0;
        let wins = self.trades.iter().filter(|t| t.pnl_percent > 0.0).count();
        let win_rate = if self.trades.is_empty() {
            0.0
        } else {
            (wins as f64 / self.trades.len() as f64) * 100.0
        };

        println!("\n=== Backtest Results: {} ===", symbol);
        println!("Trades Executed : {}", self.trades.len());
        println!("Win Rate        : {:.2}%", win_rate);
        println!("Initial Capital : ₹{:.2}", self.initial_capital);
        println!("Final Capital   : ₹{:.2}", self.final_capital);
        println!("Total Return    : {:.2}%\n", return_pct);
    }
}

pub fn run_backtest(
    candles: &[Candle],
    strategy: &dyn Strategy,
    initial_capital: f64,
) -> BacktestResult {
    let mut capital = initial_capital;
    let mut position: Option<(f64, f64)> = None;
    let mut trades = Vec::new();

    let warmup = 30;

    if candles.len() < warmup {
        return BacktestResult {
            initial_capital,
            final_capital: capital,
            trades,
        };
    }

    for i in warmup..candles.len() {
        let history_slice = &candles[0..=i];
        let current_candle = &candles[i];

        let signal = strategy.check_signal(history_slice);

        match signal {
            Signal::Buy if position.is_none() => {
                let qty = capital / current_candle.close;
                position = Some((current_candle.close, qty));
            }
            Signal::Sell if position.is_some() => {
                let (entry_price, qty) = position.unwrap();
                let exit_price = current_candle.close;

                capital = qty * exit_price;
                let pnl_percent = ((exit_price - entry_price) / entry_price) * 100.0;

                trades.push(Trade {
                    entry_price,
                    exit_price,
                    pnl_percent,
                });
                position = None;
            }
            _ => {}
        }
    }

    if let Some((entry_price, qty)) = position {
        let exit_price = candles.last().unwrap().close;
        capital = qty * exit_price;
        trades.push(Trade {
            entry_price,
            exit_price,
            pnl_percent: ((exit_price - entry_price) / entry_price) * 100.0,
        });
    }

    BacktestResult {
        initial_capital,
        final_capital: capital,
        trades,
    }
}
