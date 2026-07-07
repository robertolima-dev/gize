//! Application modules. `gize make app <name>` registers modules here.

use axum::Router;

use crate::state::AppState;

// gize:modules (do not remove this marker)
mod messages;
mod users;
mod ws;

/// Merge every module's routes. `gize make app` extends this function.
pub fn routes() -> Router<AppState> {
    Router::new()
        .merge(users::routes())
        .merge(ws::routes())
        .merge(messages::routes())
    // gize:module-routes (do not remove this marker)
}
