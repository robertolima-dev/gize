//! Templates for the optional WebSocket module (`gize new --ws`, ADR-018).
//!
//! Generates a self-contained `src/app/ws/` module — a minimal, typed echo endpoint you own
//! and extend. It is wired into `app/mod.rs` like any module, so it inherits the project's API
//! prefix (served at `/ws`, or `/api/v1/ws` when the project is versioned; ADR-016).

/// `src/app/ws/mod.rs` — module root: declares the submodules and re-exports `routes`.
pub fn mod_rs() -> String {
    r#"//! WebSocket support (ADR-018): a minimal, typed echo endpoint you own and extend.

mod handler;
mod message;
mod routes;

pub use routes::routes;
"#
    .to_string()
}

/// `src/app/ws/message.rs` — the typed messages exchanged over the socket.
pub fn message_rs() -> String {
    r#"//! Typed WebSocket messages. Extend these enums to define your own protocol; the
//! `type` tag selects the variant (e.g. `{"type":"echo","text":"hi"}`).

use serde::{Deserialize, Serialize};

/// A message received from the client.
#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    Echo { text: String },
}

/// A message sent back to the client.
#[derive(Debug, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    Echo { text: String },
    Error { message: String },
}
"#
    .to_string()
}

/// `src/app/ws/handler.rs` — the upgrade handler and per-connection loop.
pub fn handler_rs() -> String {
    r#"//! The WebSocket upgrade handler and per-connection echo loop.

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::Response;

use super::message::{ClientMessage, ServerMessage};
use crate::state::AppState;

/// Upgrade the HTTP connection to a WebSocket. `AppState` is available here for auth, shared
/// channels, database access, etc. — this echo example does not use it yet.
pub async fn upgrade(ws: WebSocketUpgrade, State(_state): State<AppState>) -> Response {
    ws.on_upgrade(handle_socket)
}

/// Per-connection loop: decode each typed client message and echo it back. Extend the `match`
/// on `ClientMessage` to handle your own message types.
async fn handle_socket(mut socket: WebSocket) {
    while let Some(Ok(msg)) = socket.recv().await {
        match msg {
            Message::Text(text) => {
                let reply = match serde_json::from_str::<ClientMessage>(&text) {
                    Ok(ClientMessage::Echo { text }) => ServerMessage::Echo { text },
                    Err(err) => ServerMessage::Error {
                        message: err.to_string(),
                    },
                };
                let payload = serde_json::to_string(&reply).unwrap_or_default();
                if socket.send(Message::Text(payload)).await.is_err() {
                    break;
                }
            }
            Message::Close(_) => break,
            _ => {}
        }
    }
}
"#
    .to_string()
}

/// `src/app/ws/routes.rs` — the router fragment merged into `app::routes()`.
pub fn routes_rs() -> String {
    r#"//! WebSocket routes, merged into the app router like any module.

use axum::routing::get;
use axum::Router;

use super::handler;
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new().route("/ws", get(handler::upgrade))
}
"#
    .to_string()
}
