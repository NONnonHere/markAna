use anyhow::Result;
use clap::{Parser, Subcommand};

use std::time::Duration;
use tokio::time::sleep;

#[derive(Parser)]
#[command(name = "mark-ana")]
#[command(about = "Market Analyzer for Swing Trading", version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Ingest {
        #[arg(short, long)]
        auth_code: String,

        #[arg(short, long, default_value = "2024-01-01")]
        from: String,

        #[arg(short, long, default_value = "2024-01-01")]
        to: String,

        #[arg(short, long, default_value = "D")]
        resolution: String,
    },
    Scan {
        #[arg(short, long, default_value = "default")]
        strategy: String,
    },
    Backtest {
        #[arg(short, long, default_value = "NSE:RELIANCE-EQ")]
        ticker: String,

        #[arg(short, long = "strategy", default_value = "safe_dip")]
        strategy_name: String,

        #[arg(long)]
        trade_info: bool,
    },
    Test {
        #[arg(short, long)]
        auth_code: String,
        #[arg(short, long, default_value = "NSE:RELIANCE-EQ")]
        symbol: String,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    dotenvy::from_path("config/.env").expect("Failed to load config/.env file!");

    let cli = Cli::parse();

    match &cli.command {
        Commands::Ingest {
            auth_code,
            from,
            to,
            resolution,
        } => {
            let app_id = std::env::var("FYERS_APP_ID").expect("Missing FYERS_APP_ID");
            let secret = std::env::var("FYERS_SECRET_KEY").expect("Missing FYERS_SECRET_KEY");
            let db_url =
                std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://store.db".to_string());

            tracing::info!("Initializing Database...");
            let pool = store::schema::init_db(&db_url).await?;

            tracing::info!("Authenticating with Fyers...");
            let mut client = fyers::FyersClient::new(&app_id, &secret);
            client.generate_token(auth_code).await?;

            // Read tickers from universe.csv
            let universe = std::fs::read_to_string("config/universe.csv")
                .unwrap_or_else(|_| "NSE:RELIANCE-EQ\nNSE:SBIN-EQ".to_string());

            let tickers: Vec<&str> = universe.lines().filter(|l| !l.is_empty()).collect();

            tracing::info!("Starting ingestion for {} tickers...", tickers.len());

            for ticker in tickers {
                tracing::info!("Fetching {}...", ticker);

                match client.get_history(ticker, &resolution, &from, &to).await {
                    Ok(candles) => {
                        store::candles::insert_candles(&pool, ticker, &candles).await?;
                        tracing::info!("Saved {} candles for {}", candles.len(), ticker);
                    }
                    Err(e) => tracing::error!("Failed to fetch {}: {}", ticker, e),
                }

                sleep(Duration::from_millis(500)).await;
            }
            tracing::info!("Ingestion complete");
        }

        Commands::Scan { strategy } => {
            tracing::info!("Running scanner with strategy: {}", strategy);

            let db_url = std::env::var("DATABASE_URL").expect("Missing DATABASE_URL");
            let pool = store::schema::init_db(&db_url).await?;

            let universe = std::fs::read_to_string("config/universe.csv")
                .unwrap_or_else(|_| "NSE:RELIANCE-EQ".to_string());
            let tickers: Vec<&str> = universe.lines().filter(|l| !l.is_empty()).collect();

            println!(
                "\n{:<20} | {:<10} | {:<10} | {:<10}",
                "Symbol", "Close", "RSI (14)", "SMA (20)"
            );
            println!("{:-<59}", "");

            for ticker in tickers {
                let candles = store::candles::fetch_candles(&pool, ticker).await?;

                if candles.is_empty() {
                    tracing::warn!(
                        "No data found for {} in database. Did you ingest it?",
                        ticker
                    );
                    continue;
                }

                let closes: Vec<f64> = candles.iter().map(|c| c.close).collect();

                let rsi_14 = indicators::rsi::rsi(&closes, 14);
                let sma_20 = indicators::ma::sma(&closes, 20);

                let latest_close = closes.last().unwrap_or(&0.0);
                let latest_rsi = rsi_14.last().unwrap_or(&f64::NAN);
                let latest_sma = sma_20.last().unwrap_or(&f64::NAN);

                println!(
                    "{:<20} | {:<10.2} | {:<10.2} | {:<10.2}",
                    ticker, latest_close, latest_rsi, latest_sma
                );
            }
            println!("\nScan complete\n");
        }
        Commands::Backtest {
            ticker,
            strategy_name,
            trade_info,
        } => {
            tracing::info!("Starting backtest for {} using {}", ticker, strategy_name);

            let db_url = std::env::var("DATABASE_URL").expect("Missing DATABASE_URL");
            let pool = store::schema::init_db(&db_url).await?;

            let candles = store::candles::fetch_candles(&pool, ticker).await?;
            if candles.is_empty() {
                tracing::error!("No data found for {}. Run ingest first.", ticker);
                return Ok(());
            }

            let active_strategy: Box<dyn strategy::Strategy> = match strategy_name.as_str() {
                "rsi" => Box::new(strategy::RsiReversionStrategy),
                "safe_dip" => Box::new(strategy::TrendFilteredRsiStrategy),
                "bollinger" => Box::new(strategy::BollingerReversionStrategy),
                "vwap" => Box::new(strategy::VwapPullbackStrategy), // Add this
                "squeeze" => Box::new(strategy::VolatilitySqueezeStrategy),
                _ => {
                    tracing::error!("Unknown strategy! Use 'rsi', 'safe_dip', or 'bollinger'.");
                    return Ok(());
                }
            };

            let mut clean_candles: Vec<fyers::model::Candle> = Vec::new();
            for c in candles {
                let ist_seconds_since_midnight = (c.date + 19800) % 86400;

                if ist_seconds_since_midnight < 33300 || ist_seconds_since_midnight >= 55800 {
                    continue;
                }

                if let Some(last) = clean_candles.last_mut() {
                    if last.date == c.date {
                        *last = c;
                        continue;
                    }
                }

                clean_candles.push(c);
            }

            let initial_capital = 100000.0;
            let result =
                backtest::run_backtest(&clean_candles, active_strategy.as_ref(), initial_capital);
            result.print_metrics(&ticker, *trade_info);
        }
        Commands::Test { auth_code, symbol } => {
            let app_id =
                std::env::var("FYERS_APP_ID").expect("FYERS_APP_ID is missing from your .env file");
            let secret = std::env::var("FYERS_SECRET_KEY")
                .expect("FYERS_SECRET_KEY is missing from your .env file");

            tracing::info!("Initializing Fyers Client...");
            let mut client = fyers::FyersClient::new(&app_id, &secret);

            tracing::info!("Exchanging auth_code for access_token...");
            client.generate_token(auth_code).await?;
            tracing::info!("Token generated successfully!");

            tracing::info!("Fetching history for {}...", symbol);
            let history = client
                .get_history(symbol, "1D", "2026-09-20", "2026-09-25")
                .await?;

            println!("Successfully fetched {} candles:", history.len());
            for candle in history {
                println!("{:?}", candle);
            }
        }
    }

    Ok(())
}
