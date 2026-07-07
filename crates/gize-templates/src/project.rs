//! Templates for `gize new`: the files that make up a fresh project skeleton.

use gize_core::{Dialect, Manifest};

/// `Cargo.toml` for the generated application. The `dialect` selects the sqlx driver feature
/// (ADR-015); `websocket` enables Axum's `ws` feature and `serde_json` for typed WebSocket
/// messages (ADR-018).
pub fn cargo_toml(name: &str, dialect: Dialect, websocket: bool) -> String {
    // Axum's WebSocket support is behind its `ws` feature, and the generated `ws` module parses
    // typed messages with `serde_json`, so both are added only when `--ws` is used.
    let axum_dep = if websocket {
        "axum = { version = \"0.7\", features = [\"ws\"] }"
    } else {
        "axum = \"0.7\""
    };
    let serde_json_dep = if websocket {
        "\nserde_json = \"1\""
    } else {
        ""
    };
    format!(
        r#"[package]
name = "{name}"
version = "0.1.0"
edition = "2024"

[dependencies]
{axum_dep}
tokio = {{ version = "1", features = ["full"] }}
serde = {{ version = "1", features = ["derive"] }}{serde_json_dep}
sqlx = {{ version = "0.8", features = ["runtime-tokio", "{driver}", "uuid", "chrono"] }}
uuid = {{ version = "1", features = ["v4", "serde"] }}
chrono = {{ version = "0.4", features = ["serde"] }}
anyhow = "1"
tracing = "0.1"
tracing-subscriber = "0.3"
argon2 = {{ version = "0.5", features = ["std"] }}
jsonwebtoken = "9"
validator = {{ version = "0.19", features = ["derive"] }}
"#,
        driver = dialect.sqlx_feature(),
    )
}

/// The `gize.toml` manifest for a new project.
pub fn gize_toml(manifest: &Manifest) -> String {
    manifest
        .to_toml()
        .unwrap_or_else(|_| String::from("# failed to render manifest\n"))
}

/// `.env.example` — runtime config lives in the environment (ADR-009). The `DATABASE_URL`
/// example follows the selected dialect (ADR-015).
pub fn env_example(name: &str, dialect: Dialect) -> String {
    format!(
        "# Runtime configuration for {name} (copy to .env)\n\
         DATABASE_URL={url}\n\
         PORT=8080\n\
         # Secret used to sign auth tokens (JWT/HS256). Use a long random value in production.\n\
         GIZE_JWT_SECRET=dev-only-change-me\n",
        url = dialect.example_url(name),
    )
}

/// `src/main.rs`: binary entrypoint that builds state and serves the router.
pub fn main_rs() -> String {
    r#"mod app;
mod auth;
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

/// `src/state.rs`: shared application state (DB pool + config). The pool type follows the
/// selected dialect (ADR-015).
pub fn state_rs(dialect: Dialect) -> String {
    r#"use anyhow::Result;
use sqlx::__MODULE__::__POOL_OPTIONS__;
use sqlx::__POOL__;

/// Shared application state, injected into every handler.
#[derive(Clone)]
pub struct AppState {
    // Read by generated repositories once you add a resource (`gize make crud`).
    // Allowed to be unused so a freshly scaffolded, model-less app stays clippy-clean.
    #[allow(dead_code)]
    pub db: __POOL__,
}

impl AppState {
    /// Build state from environment variables (12-factor; see ADR-009).
    pub async fn from_env() -> Result<Self> {
        let url = std::env::var("DATABASE_URL")
            .map_err(|_| anyhow::anyhow!("DATABASE_URL must be set"))?;
        let db = __POOL_OPTIONS__::new()
            .max_connections(5)
            .connect(&url)
            .await?;
        Ok(Self { db })
    }
}
"#
    .replace("__MODULE__", dialect.sqlx_module())
    .replace("__POOL_OPTIONS__", dialect.pool_options())
    .replace("__POOL__", dialect.pool_type())
}

/// `src/router.rs`: top-level router that mounts each module's routes.
/// `src/router.rs`: builds the top-level router. When `api_mount` is `Some` (a versioned
/// project, ADR-016) the app is nested under it (e.g. `/api/v1`); otherwise it is merged at
/// the root, byte-identical to an unversioned project.
pub fn router_rs(api_mount: Option<&str>) -> String {
    let routes_line = match api_mount {
        Some(mount) => format!(".nest(\"{mount}\", app::routes())"),
        None => ".merge(app::routes())".to_string(),
    };
    format!(
        r#"use axum::Router;

use crate::app;
use crate::state::AppState;

/// Build the application router. `gize make app` registers module routers here.
pub fn build(state: AppState) -> Router {{
    Router::new()
        // gize:routes (do not remove this marker)
        {routes_line}
        .with_state(state)
}}
"#
    )
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
