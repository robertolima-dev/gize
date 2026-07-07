//! The chat message exchanged over the socket. The client sends it as JSON
//! (`{"username":"ana","content":"hi"}`); the server persists it and broadcasts the same shape
//! back to every connected client.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub username: String,
    pub content: String,
}
