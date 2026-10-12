use chrono::TimeZone;
use fyers::model::Candle;
use strategy::{Signal, Strategy};

pub struct Trade {
    pub trade_type: String,
    pub entry_time: i64,
    pub exit_time: i64,
    pub entry_price: f64,
    pub exit_price: f64,
    pub pnl_percent: f64,
    pub stop_loss: f64,
}

pub struct BacktestResult {
    pub initial_capital: f64,
    pub final_capital: f64,
    pub trades: Vec<Trade>,
}

impl BacktestResult {
    pub fn print_metrics(&self, symbol: &str, show_trades: bool) {
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

        if show_trades && !self.trades.is_empty() {
            println!("Detailed Trade Log");
            for (i, trade) in self.trades.iter().enumerate() {
                let status = if trade.pnl_percent > 0.0 {
                    "WIN "
                } else {
                    "LOSS"
                };
                println!(
                    "[{}] [{}] Trade {:02}: Entry: ₹{:.2} | Exit: ₹{:.2} | PnL: {:+.2}%",
                    trade.trade_type,
                    status,
                    i + 1,
                    trade.entry_price,
                    trade.exit_price,
                    trade.pnl_percent
                );
            }
            println!("---\n");
        }
    }
}

use std::fs::File;
use std::io::Write;

pub fn generate_trade_chart(candles: &[Candle], trade: &Trade, trade_id: usize) {
    let mut chart_data = String::new();
    let mut last_time = 0;

    for c in candles {
        if c.open == 0.0 || c.high == 0.0 || c.low == 0.0 || c.close == 0.0 {
            continue;
        }

        if c.date <= last_time {
            continue;
        }
        last_time = c.date;

        chart_data.push_str(&format!(
            "{{ time: {}, open: {}, high: {}, low: {}, close: {} }},\n",
            c.date, c.open, c.high, c.low, c.close
        ));
    }

    let html = format!(
        r#"
        <!DOCTYPE html>
        <html>
        <head>
            <title>Trade {} Visualizer</title>
            <script src="https://unpkg.com/lightweight-charts@4.1.1/dist/lightweight-charts.standalone.production.js"></script>
            <style>
                body {{ background: #131722; margin: 0; padding: 0; }}
                #tvchart {{ width: 100vw; height: 100vh; }}
            </style>
        </head>
        <body>
            <div id="tvchart"></div>
            <script>
                const chart = LightweightCharts.createChart(document.getElementById('tvchart'), {{
                    layout: {{ background: {{ color: '#131722' }}, textColor: '#d1d4dc' }},
                    grid: {{ vertLines: {{ color: '#2b2b43' }}, horzLines: {{ color: '#2b2b43' }} }},
                    timeScale: {{ timeVisible: true, secondsVisible: false }}
                }});

                const candleSeries = chart.addCandlestickSeries();
                candleSeries.setData([
                    {}
                ]);

                // 1. Entry Line (Blue)
                candleSeries.createPriceLine({{
                    price: {},
                    color: '#2196F3',
                    lineWidth: 2,
                    lineStyle: 2,
                    axisLabelVisible: true,
                    title: 'Entry'
                }});

                // 2. Exit Line (Pink)
                candleSeries.createPriceLine({{
                    price: {},
                    color: '#e91e63',
                    lineWidth: 2,
                    lineStyle: 2,
                    axisLabelVisible: true,
                    title: 'Exit'
                }});

                // 3. Stop Loss Line (Orange)
                candleSeries.createPriceLine({{
                    price: {},
                    color: '#ff9800',
                    lineWidth: 2,
                    lineStyle: 1,
                    axisLabelVisible: true,
                    title: 'Stop Loss'
                }});
            </script>
        </body>
        </html>
        "#,
        trade_id,          //Matches <title>
        chart_data,        //Matches setData()
        trade.entry_price, //Matches Entry Line
        trade.exit_price,  //Matches Exit Line
        trade.stop_loss    //Matches Stop Loss Line
    );

    let filename = format!("trade_{:02}_chart.html", trade_id);
    let mut file = std::fs::File::create(&filename).unwrap();
    std::io::Write::write_all(&mut file, html.as_bytes()).unwrap();
}

pub fn run_backtest(
    candles: &[Candle],
    strategy: &dyn Strategy,
    initial_capital: f64,
) -> BacktestResult {
    let mut capital = initial_capital;
    let mut position: Option<(i64, f64, f64)> = None;
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

        let ist_seconds = (current_candle.date + 19800) % 86400;

        if ist_seconds >= 54900 && position.is_some() {
            let (entry_time, entry_price, qty) = position.unwrap();
            let exit_time = current_candle.date;
            let exit_price = current_candle.close;
            capital = qty * exit_price;

            trades.push(Trade {
                trade_type: "LONG".to_string(),
                entry_time,
                exit_time,
                entry_price,
                exit_price,
                pnl_percent: ((exit_price - entry_price) / entry_price) * 100.0,
                stop_loss: entry_price * (1.0 - 0.003),
            });
            position = None;

            let context_start = if i > 100 { i - 100 } else { 0 };
            generate_trade_chart(
                &candles[context_start..=i],
                trades.last().unwrap(),
                trades.len(),
            );
            continue;
        }

        if ist_seconds >= 54000 && position.is_none() {
            continue;
        }

        let entry_price = position.map(|(_, price, _)| price);
        let signal = strategy.check_signal(history_slice, entry_price);

        match signal {
            Signal::Buy if position.is_none() => {
                let qty = capital / current_candle.close;
                // Save the current_candle.date
                position = Some((current_candle.date, current_candle.close, qty));
            }
            Signal::Sell if position.is_some() => {
                let (entry_time, entry_price, qty) = position.unwrap();
                let exit_time = current_candle.date;
                let exit_price = current_candle.close;

                capital = qty * exit_price;
                let pnl_percent = ((exit_price - entry_price) / entry_price) * 100.0;

                trades.push(Trade {
                    trade_type: "LONG".to_string(),
                    entry_time, // Add this
                    exit_time,  // Add this
                    entry_price,
                    exit_price,
                    pnl_percent,
                    stop_loss: entry_price * (1.0 - 0.003),
                });
                position = None;

                let context_start = if i > 100 { i - 100 } else { 0 };
                let chart_candles = &candles[context_start..=i];
                generate_trade_chart(chart_candles, trades.last().unwrap(), trades.len());
            }
            _ => {}
        }
    }

    if let Some((entry_time, entry_price, qty)) = position {
        let last_index = candles.len() - 1;
        let last_candle = &candles[last_index];
        let exit_price = last_candle.close;
        let exit_time = last_candle.date;

        capital = qty * exit_price;
        trades.push(Trade {
            trade_type: "LONG".to_string(),
            entry_time,
            exit_time,
            entry_price,
            exit_price,
            pnl_percent: ((exit_price - entry_price) / entry_price) * 100.0,
            stop_loss: entry_price * (1.0 - 0.003),
        });

        // Generate the chart for this final force-closed trade
        let context_start = if last_index > 100 {
            last_index - 100
        } else {
            0
        };
        let chart_candles = &candles[context_start..=last_index];
        generate_trade_chart(chart_candles, trades.last().unwrap(), trades.len());
    }

    BacktestResult {
        initial_capital,
        final_capital: capital,
        trades,
    }
}
