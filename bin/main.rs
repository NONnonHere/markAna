use anyhow::Result;
use clap::{Parser, Subcommand};

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
        ticker: Option<String>,
        #[arg(short, long)]
        from: Option<String>,
    },
    Scan {
        #[arg(short, long, default_value = "default")]
        strategy: String,
    },
    Backtest {
        #[arg(short, long)]
        config: String,
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
        Commands::Ingest { ticker: _, from: _ } => {
            tracing::info!("Starting ingestion pipeline...");
        }
        Commands::Scan { strategy: _ } => {
            tracing::info!("Running scanner...");
        }
        Commands::Backtest { config: _ } => {
            tracing::info!("Running backtest...");
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
                .get_history(symbol, "1D", "2024-01-01", "2024-01-10")
                .await?;

            println!("Successfully fetched {} candles:", history.len());
            for candle in history {
                println!("{:?}", candle);
            }
        }
    }

    Ok(())
}
