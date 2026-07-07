use axum::routing::{get, post, put};
use axum::{Router, middleware};

use super::handler;
use crate::auth::require_auth;
use crate::state::AppState;

/// Routes for the `users` resource. `register`/`login` and reads are public; writes require
/// a valid bearer token.
pub fn routes() -> Router<AppState> {
    let public = Router::new()
        .route("/users/register", post(handler::register))
        .route("/users/login", post(handler::login))
        .route("/users", get(handler::list))
        .route("/users/:id", get(handler::show));

    let protected = Router::new()
        .route("/users", post(handler::create))
        .route("/users/:id", put(handler::update).delete(handler::delete))
        .route_layer(middleware::from_fn(require_auth));

    public.merge(protected)
}
