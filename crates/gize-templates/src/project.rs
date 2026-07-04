//! Templates for `gize new`: the files that make up a fresh project skeleton.

use gize_core::Manifest;

/// `Cargo.toml` for the generated application.
pub fn cargo_toml(name: &str) -> String {
    format!(
        r#"[package]
name = "{name}"
version = "0.1.0"
edition = "2024"

[dependencies]
axum = "0.7"
tokio = {{ version = "1", features = ["full"] }}
serde = {{ version = "1", features = ["derive"] }}
sqlx = {{ version = "0.8", features = ["runtime-tokio", "postgres", "uuid", "chrono"] }}
uuid = {{ version = "1", features = ["v4", "serde"] }}
chrono = {{ version = "0.4", features = ["serde"] }}
anyhow = "1"
tracing = "0.1"
tracing-subscriber = "0.3"
"#
    )
}

/// The `gize.toml` manifest for a new project.
pub fn gize_toml(manifest: &Manifest) -> String {
    manifest
        .to_toml()
        .unwrap_or_else(|_| String::from("# failed to render manifest\n"))
}

/// `.env.example` — runtime config lives in the environment (ADR-009).
pub fn env_example(name: &str) -> String {
    format!(
        "# Runtime configuration for {name} (copy to .env)\n\
         DATABASE_URL=postgres://postgres:postgres@localhost:5432/{name}\n\
         PORT=8080\n"
    )
}

/// `src/main.rs`: binary entrypoint that builds state and serves the router.
pub fn main_rs() -> String {
    r#"mod app;
mod config;
mod router;
mod state;

use anyhow::Result;

use config::Config;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let config = Config::from_env();
    let state = state::AppState::from_env().await?;
    let app = router::build(state);

    let addr = format!("0.0.0.0:{}", config.port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("gize app listening on {addr}");
    axum::serve(listener, app).await?;
    Ok(())
}
"#
    .to_string()
}

/// `src/state.rs`: shared application state (DB pool + config).
pub fn state_rs() -> String {
    r#"use anyhow::Result;
use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

/// Shared application state, injected into every handler.
#[derive(Clone)]
pub struct AppState {
    // Read by generated repositories once you add a resource (`gize make crud`).
    // Allowed to be unused so a freshly scaffolded, model-less app stays clippy-clean.
    #[allow(dead_code)]
    pub db: PgPool,
}

impl AppState {
    /// Build state from environment variables (12-factor; see ADR-009).
    pub async fn from_env() -> Result<Self> {
        let url = std::env::var("DATABASE_URL")
            .map_err(|_| anyhow::anyhow!("DATABASE_URL must be set"))?;
        let db = PgPoolOptions::new()
            .max_connections(5)
            .connect(&url)
            .await?;
        Ok(Self { db })
    }
}
"#
    .to_string()
}

/// `src/router.rs`: top-level router that mounts each module's routes.
pub fn router_rs() -> String {
    r#"use axum::Router;

use crate::app;
use crate::state::AppState;

/// Build the application router. `gize make app` registers module routers here.
pub fn build(state: AppState) -> Router {
    Router::new()
        // gize:routes (do not remove this marker)
        .merge(app::routes())
        .with_state(state)
}
"#
    .to_string()
}

/// `src/config/mod.rs`: typed runtime configuration.
pub fn config_mod_rs() -> String {
    r#"//! Runtime configuration loaded from the environment.

/// Application configuration. Extend as needed.
#[derive(Debug, Clone)]
pub struct Config {
    pub port: u16,
}

impl Config {
    pub fn from_env() -> Self {
        let port = std::env::var("PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(8080);
        Self { port }
    }
}
"#
    .to_string()
}

/// `src/app/mod.rs`: aggregates the application modules. `gize make app` edits this file.
pub fn app_mod_rs() -> String {
    r#"//! Application modules. `gize make app <name>` registers modules here.

use axum::Router;

use crate::state::AppState;

// gize:modules (do not remove this marker)

/// Merge every module's routes. `gize make app` extends this function.
pub fn routes() -> Router<AppState> {
    Router::new()
    // gize:module-routes (do not remove this marker)
}
"#
    .to_string()
}
