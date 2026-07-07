//! WebSocket routes, merged into the app router like any module.

use axum::Router;
use axum::routing::get;

use super::handler;
use crate::state::AppState;

pub fn routes() -> Router<AppState> {
    Router::new()
        // The chat client page, so `http://localhost:8080/` is a working demo.
        .route("/", get(handler::index))
        .route("/ws", get(handler::upgrade))
}
