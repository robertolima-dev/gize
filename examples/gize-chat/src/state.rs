use anyhow::Result;
use sqlx::SqlitePool;
use sqlx::sqlite::SqlitePoolOptions;
use tokio::sync::broadcast;

/// Shared application state, injected into every handler.
///
/// Hand-extended (this is generated code you own): besides the SQLx pool, it carries a
/// `broadcast` channel so every WebSocket connection can fan a chat message out to all the
/// others. See `src/app/ws/handler.rs`.
#[derive(Clone)]
pub struct AppState {
    pub db: SqlitePool,
    /// Sender side of the chat broadcast; each connection subscribes for a receiver.
    pub tx: broadcast::Sender<String>,
}

impl AppState {
    /// Build state from environment variables (12-factor; see ADR-009).
    pub async fn from_env() -> Result<Self> {
        let url = std::env::var("DATABASE_URL")
            .map_err(|_| anyhow::anyhow!("DATABASE_URL must be set"))?;
        let db = SqlitePoolOptions::new()
            .max_connections(5)
            .connect(&url)
            .await?;
        // Buffer a few messages so a briefly-slow client does not drop the stream immediately.
        let (tx, _rx) = broadcast::channel(128);
        Ok(Self { db, tx })
    }
}
