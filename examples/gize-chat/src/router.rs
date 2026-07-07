use axum::Router;

use crate::app;
use crate::state::AppState;

/// Build the application router. `gize make app` registers module routers here.
pub fn build(state: AppState) -> Router {
    Router::new()
        // gize:routes (do not remove this marker)
        .merge(app::routes())
        .with_state(state)
}
