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
    pub kelly_fraction: f64,
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
                    "[{}] [{}] Trade {:02}: Entry: ₹{:.2} | Exit: ₹{:.2} | PnL: {:+.2}% | Size: {:.0}%",
                    trade.trade_type,
                    status,
                    i + 1,
                    trade.entry_price,
                    trade.exit_price,
                    trade.pnl_percent,
                    trade.kelly_fraction * 100.0
                );
            }
            println!("---\n");
        }
    }
}

pub fn generate_trade_chart(candles: &[Candle], sma_vals: &[f64], trade: &Trade, trade_id: usize) {
    let mut chart_data = String::new();
    let mut sma_data = String::new();
    let mut last_time = 0;

    for (i, c) in candles.iter().enumerate() {
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

        let val = sma_vals[i];
        if !val.is_nan() && val > 0.0 {
            sma_data.push_str(&format!("{{ time: {}, value: {} }},\n", c.date, val));
        }
    }

    let is_long = trade.trade_type == "LONG";
    let entry_shape = if is_long { "arrowUp" } else { "arrowDown" };
    let entry_pos = if is_long { "belowBar" } else { "aboveBar" };
    let exit_shape = if is_long { "arrowDown" } else { "arrowUp" };
    let exit_pos = if is_long { "aboveBar" } else { "belowBar" };

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
                    timeScale: {{ timeVisible: true, secondsVisible: false }},
                    localization: {{
                        timeFormatter: timestamp => {{
                            return new Date(timestamp * 1000).toLocaleTimeString('en-IN', {{
                                timeZone: 'Asia/Kolkata', hour: '2-digit', minute: '2-digit', hour12: false
                            }});
                        }}
                    }}
                }});

                const candleSeries = chart.addCandlestickSeries();
                candleSeries.setData([
                    {}
                ]);

                // Macro Trend Line (200 SMA)
                const smaSeries = chart.addLineSeries({{ color: '#ffeb3b', lineWidth: 2, title: '200 SMA' }});
                smaSeries.setData([
                    {}
                ]);

                candleSeries.setMarkers([
                    {{ time: {}, position: '{}', color: '#2196F3', shape: '{}', size: 1 }},
                    {{ time: {}, position: '{}', color: '#e91e63', shape: '{}', size: 1 }}
                ]);

                candleSeries.createPriceLine({{ price: {}, color: '#2196F3', lineWidth: 1, lineStyle: 2, axisLabelVisible: true, title: 'Entry' }});
                candleSeries.createPriceLine({{ price: {}, color: '#e91e63', lineWidth: 1, lineStyle: 2, axisLabelVisible: true, title: 'Exit' }});
                candleSeries.createPriceLine({{ price: {}, color: '#ff9800', lineWidth: 1, lineStyle: 1, axisLabelVisible: true, title: 'Stop Loss' }});
            </script>
        </body>
        </html>
        "#,
        trade_id,
        chart_data,
        sma_data,
        trade.entry_time,
        entry_pos,
        entry_shape,
        trade.exit_time,
        exit_pos,
        exit_shape,
        trade.entry_price,
        trade.exit_price,
        trade.stop_loss
    );

    let filename = format!("trade_{:02}_chart.html", trade_id);
    let mut file = std::fs::File::create(&filename).unwrap();
    std::io::Write::write_all(&mut file, html.as_bytes()).unwrap();
}

pub fn run_backtest(
    candles: &[Candle],
    strat: &dyn Strategy,
    initial_capital: f64,
) -> BacktestResult {
    let mut capital = initial_capital;
    let mut position: Option<(String, i64, f64, f64, f64, f64)> = None;
    let mut trades = Vec::new();

    let closes: Vec<f64> = candles.iter().map(|c| c.close).collect();
    let full_sma = strategy::sma(&closes, 200);

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
            let (trade_type, entry_time, entry_price, qty, stop_loss, kelly_pct) =
                position.unwrap();
            let exit_time = current_candle.date;
            let exit_price = current_candle.close;

            let pnl_percent = if trade_type == "LONG" {
                capital += (exit_price - entry_price) * qty;
                ((exit_price - entry_price) / entry_price) * 100.0
            } else {
                capital += (entry_price - exit_price) * qty;
                ((entry_price - exit_price) / entry_price) * 100.0
            };

            trades.push(Trade {
                trade_type,
                entry_time,
                exit_time,
                entry_price,
                exit_price,
                pnl_percent,
                stop_loss,
                kelly_fraction: kelly_pct,
            });
            position = None;

            let context_start = if i > 100 { i - 100 } else { 0 };
            let context_end = std::cmp::min(i + 30, candles.len() - 1);
            generate_trade_chart(
                &candles[context_start..=context_end],
                &full_sma[context_start..=context_end],
                trades.last().unwrap(),
                trades.len(),
            );
            continue;
        }

        if ist_seconds >= 54000 && position.is_none() {
            continue;
        }

        let strategy_pos = position
            .as_ref()
            .map(|(t, _, p, _, _, _)| if t == "LONG" { *p } else { -*p });
        let signal = strat.check_signal(history_slice, strategy_pos);
        let mut kelly_fraction = 0.25;

        if trades.len() >= 5 {
            let window: Vec<&Trade> = trades.iter().rev().take(20).collect();
            let count = window.len() as f64;
            let wins = window.iter().filter(|t| t.pnl_percent > 0.0).count() as f64;
            let w = wins / count;

            let avg_win = window
                .iter()
                .filter(|t| t.pnl_percent > 0.0)
                .map(|t| t.pnl_percent)
                .sum::<f64>()
                / wins.max(1.0);
            let losses = count - wins;
            let avg_loss = window
                .iter()
                .filter(|t| t.pnl_percent <= 0.0)
                .map(|t| t.pnl_percent.abs())
                .sum::<f64>()
                / losses.max(1.0);

            if avg_loss > 0.0 && avg_win > 0.0 {
                let r = avg_win / avg_loss;
                let kelly = w - ((1.0 - w) / r);

                kelly_fraction = (kelly * 0.5).clamp(0.2, 1.0);
            }
        }
        match signal {
            Signal::Buy if position.is_none() => {
                let qty = capital / current_candle.close;
                let atr_vals = strategy::atr(history_slice, 14);
                let entry_atr = atr_vals.last().unwrap_or(&0.0);
                let stop_loss = current_candle.close - (2.0 * entry_atr);
                position = Some((
                    "LONG".to_string(),
                    current_candle.date,
                    current_candle.close,
                    qty,
                    stop_loss,
                    kelly_fraction,
                ));
            }
            Signal::Short if position.is_none() => {
                let qty = capital / current_candle.close;
                let atr_vals = strategy::atr(history_slice, 14);
                let entry_atr = atr_vals.last().unwrap_or(&0.0);
                let stop_loss = current_candle.close + (2.0 * entry_atr);
                position = Some((
                    "SHORT".to_string(),
                    current_candle.date,
                    current_candle.close,
                    qty,
                    stop_loss,
                    kelly_fraction,
                ));
            }
            Signal::Sell | Signal::Cover if position.is_some() => {
                let (trade_type, entry_time, entry_price, qty, stop_loss, kelly_pct) =
                    position.unwrap();
                let exit_time = current_candle.date;
                let exit_price = current_candle.close;

                let pnl_percent = if trade_type == "LONG" {
                    capital += (exit_price - entry_price) * qty;
                    ((exit_price - entry_price) / entry_price) * 100.0
                } else {
                    capital += (entry_price - exit_price) * qty;
                    ((entry_price - exit_price) / entry_price) * 100.0
                };

                trades.push(Trade {
                    trade_type,
                    entry_time,
                    exit_time,
                    entry_price,
                    exit_price,
                    pnl_percent,
                    stop_loss,
                    kelly_fraction: kelly_pct,
                });
                position = None;

                let context_start = if i > 100 { i - 100 } else { 0 };
                let context_end = std::cmp::min(i + 30, candles.len() - 1);
                generate_trade_chart(
                    &candles[context_start..=context_end],
                    &full_sma[context_start..=context_end],
                    trades.last().unwrap(),
                    trades.len(),
                );
            }
            _ => {}
        }
    }

    if let Some((trade_type, entry_time, entry_price, qty, stop_loss, kelly_pct)) = position {
        let last_index = candles.len() - 1;
        let exit_price = candles[last_index].close;
        let exit_time = candles[last_index].date;

        let pnl_percent = if trade_type == "LONG" {
            capital += (exit_price - entry_price) * qty;
            ((exit_price - entry_price) / entry_price) * 100.0
        } else {
            capital += (entry_price - exit_price) * qty;
            ((entry_price - exit_price) / entry_price) * 100.0
        };

        trades.push(Trade {
            trade_type,
            entry_time,
            exit_time,
            entry_price,
            exit_price,
            pnl_percent,
            stop_loss,
            kelly_fraction: kelly_pct,
        });

        // Generate the chart for this final force-closed trade
        let context_start = if last_index > 100 {
            last_index - 100
        } else {
            0
        };
        generate_trade_chart(
            &candles[context_start..=last_index],
            &full_sma[context_start..=last_index],
            trades.last().unwrap(),
            trades.len(),
        );
    }

    BacktestResult {
        initial_capital,
        final_capital: capital,
        trades,
    }
}
