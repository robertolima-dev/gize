//! WebSocket support (ADR-018): a minimal, typed echo endpoint you own and extend.

mod handler;
mod message;
mod routes;

pub use routes::routes;
