use anyhow::Result;
use fyers::model::Candle;
use sqlx::{Row, SqlitePool};

pub async fn insert_candles(pool: &SqlitePool, symbol: &str, candles: &[Candle]) -> Result<()> {
    let mut tx = pool.begin().await?;

    for c in candles {
        sqlx::query(
            "INSERT OR IGNORE INTO candles (symbol, timestamp, open, high, low, close, volume)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        )
        .bind(symbol)
        .bind(c.date)
        .bind(c.open)
        .bind(c.high)
        .bind(c.low)
        .bind(c.close)
        .bind(c.volume)
        .execute(&mut *tx)
        .await?;
    }

    tx.commit().await?;
    Ok(())
}

pub async fn fetch_candles(pool: &SqlitePool, symbol: &str) -> Result<Vec<Candle>> {
    let records = sqlx::query(
        "SELECT timestamp, open, high, low, close, volume
         FROM candles
         WHERE symbol = ?1
         ORDER BY timestamp ASC",
    )
    .bind(symbol)
    .fetch_all(pool)
    .await?;

    let candles = records
        .into_iter()
        .map(|r| Candle {
            date: r.get("timestamp"),
            open: r.get("open"),
            high: r.get("high"),
            low: r.get("low"),
            close: r.get("close"),
            volume: r.get("volume"),
        })
        .collect();

    Ok(candles)
}
