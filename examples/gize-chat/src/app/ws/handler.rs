//! The WebSocket upgrade handler and per-connection chat loop.
//!
//! Hand-extended from the generated echo starter: instead of echoing to the one sender, each
//! connection now **persists** the message (through the generated `messages` resource) and
//! **broadcasts** it to every other connection via the `AppState` broadcast channel.

use axum::extract::State;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::{Html, Response};

use super::message::ChatMessage;
use crate::app::messages::dto::CreateMessage;
use crate::app::messages::service;
use crate::state::AppState;

/// The chat client page, served at `/`.
pub async fn index() -> Html<&'static str> {
    Html(include_str!("client.html"))
}

/// Upgrade the HTTP connection to a WebSocket and join the chat.
pub async fn upgrade(ws: WebSocketUpgrade, State(state): State<AppState>) -> Response {
    ws.on_upgrade(move |socket| handle_socket(socket, state))
}

/// One connection. A single task drives both directions with `tokio::select!`, so no `futures`
/// stream split is needed: it forwards broadcast messages to this client, and persists then
/// broadcasts the messages this client sends.
async fn handle_socket(mut socket: WebSocket, state: AppState) {
    let mut rx = state.tx.subscribe();
    loop {
        tokio::select! {
            // A frame from this client.
            incoming = socket.recv() => {
                match incoming {
                    Some(Ok(Message::Text(text))) => {
                        let Ok(msg) = serde_json::from_str::<ChatMessage>(&text) else {
                            continue; // ignore malformed frames
                        };
                        // Persist through the generated resource, then fan out on success.
                        let input = CreateMessage {
                            username: msg.username.clone(),
                            content: msg.content.clone(),
                        };
                        if service::create(&state.db, &input).await.is_ok() {
                            let _ = state.tx.send(serde_json::to_string(&msg).unwrap_or_default());
                        }
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Err(_)) => break,
                    Some(Ok(_)) => {} // ping/pong/binary: ignore
                }
            }
            // A message broadcast by another connection (a receive error just means we lagged
            // behind the buffer, so skip it and keep going).
            broadcast = rx.recv() => {
                let Ok(payload) = broadcast else { continue };
                if socket.send(Message::Text(payload)).await.is_err() {
                    break;
                }
            }
        }
    }
}
