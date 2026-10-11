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

        #[arg(short, long, default_value = "202-01-01")]
        from: String,

        #[arg(short, long, default_value = "2024-01-01")]
        to: String,
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
        Commands::Ingest { auth_code, from, to } => {
                    tracing::info!("Initializing Database...");
                    let db_url = std::env::var("DATABASE_URL").expect("Missing DATABASE_URL");
                    let pool = store::schema::init_db(&db_url).await?;

                    tracing::info!("Authenticating with Fyers...");
                    let app_id = std::env::var("FYERS_APP_ID").expect("Missing FYERS_APP_ID in .env");
                    let secret_key = std::env::var("FYERS_SECRET_KEY").expect("Missing FYERS_SECRET_KEY in .env");

                    let mut client = fyers::FyersClient::new(&app_id, &secret_key);
                    client.generate_token(&auth_code).await?;
                    tracing::info!("Token generated successfully!");

                    // Calculate safe date range (< 365 days) if from/to are not passed or are invalid
                    let today = chrono::Local::now();
                                let range_from = (today - chrono::Duration::days(340)).format("%Y-%m-%d").to_string();
                                let range_to = today.format("%Y-%m-%d").to_string();

                    // Read tickers from universe.csv
                    let csv_data = std::fs::read_to_string("universe.csv")
                        .expect("Could not read universe.csv. Make sure it exists in the project root!");

                    let mut tickers = Vec::new();
                    for line in csv_data.lines().skip(1) {
                        let sym = line.split(',').next().unwrap_or("").trim();
                        if !sym.is_empty() {
                            tickers.push(sym.to_string());
                        }
                    }

                    tracing::info!(
                        "Starting ingestion for {} tickers (from {} to {})...",
                        tickers.len(),
                        range_from,
                        range_to
                    );

                    for ticker in tickers {
                        tracing::info!("Fetching {}...", ticker);

                        match client.get_history(&ticker, "D", &range_from, &range_to).await {
                            Ok(candles) => {
                                if !candles.is_empty() {
                                    store::candles::insert_candles(&pool, &ticker, &candles).await?;
                                    tracing::info!("Saved {} candles for {}", candles.len(), ticker);
                                } else {
                                    tracing::warn!("No candles returned for {}", ticker);
                                }
                            }
                            Err(e) => {
                                tracing::error!("Failed to fetch {}: {}", ticker, e);
                            }
                        }

                        tokio::time::sleep(tokio::time::Duration::from_millis(300)).await;
                    }

                    tracing::info!("Batch ingestion complete!");
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
        Commands::Backtest { ticker, strategy_name } => {
                    let db_url = std::env::var("DATABASE_URL").expect("Missing DATABASE_URL");
                    let pool = store::schema::init_db(&db_url).await?;

                    // 1. Determine which stocks to run
                    let mut tickers_to_run = Vec::new();
                    if ticker == "ALL" {
                        tracing::info!("Batch mode activated. Reading universe.csv...");
                        let csv_data = std::fs::read_to_string("universe.csv").expect("Could not read universe.csv");
                        for line in csv_data.lines().skip(1) { // Skip header row
                            let symbol = line.split(',').next().unwrap_or("").trim();
                            if !symbol.is_empty() {
                                tickers_to_run.push(symbol.to_string());
                            }
                        }
                    } else {
                        tickers_to_run.push(ticker.clone());
                    }

                    // 2. Initialize the Strategy once
                    let active_strategy: Box<dyn strategy::Strategy> = match strategy_name.as_str() {
                        "rsi" => Box::new(strategy::RsiReversionStrategy),
                        "safe_dip" => Box::new(strategy::TrendFilteredRsiStrategy),
                        "bollinger" => Box::new(strategy::BollingerReversionStrategy),
                        "kronos" => Box::new(strategy::KronosStrategy::default()),
                        _ => {
                            tracing::error!("Unknown strategy! Use 'rsi', 'safe_dip', 'bollinger', or 'kronos'.");
                            return Ok(());
                        }
                    };

                    // 3. Loop through the universe
                    let initial_capital = 100_000.0;
                    for t in tickers_to_run {
                        tracing::info!("Running {} on {}", strategy_name, t);

                        let candles = store::candles::fetch_candles(&pool, &t).await?;
                        if candles.is_empty() {
                            tracing::warn!("No data found for {}. Skipping.", t);
                            continue;
                        }

                        let result = backtest::run_backtest(&candles, active_strategy.as_ref(), initial_capital);
                        result.print_metrics(&t);
                    }
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
