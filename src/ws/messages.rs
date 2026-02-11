//! WebSocket message types: envelope, commands, and events.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Top-level WebSocket message envelope.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WsMessage {
    /// Client-provided ID for requests; server-generated for events.
    pub id: String,
    /// Message type discriminator.
    #[serde(rename = "type")]
    pub msg_type: WsMessageType,
    /// ISO-8601 timestamp.
    pub timestamp: DateTime<Utc>,
    /// Variant-specific payload.
    pub payload: serde_json::Value,
}

/// Discriminator for WebSocket message types.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WsMessageType {
    /// Client → Server command.
    Command,
    /// Server → Client response to a command.
    Response,
    /// Server → Client broadcast event.
    Event,
    /// Server → Client error.
    Error,
}

/// Commands that a client can send over WebSocket.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "command", rename_all = "snake_case")]
pub enum WsCommand {
    /// Subscribe to events for specific pools.
    Subscribe {
        /// Pool IDs to subscribe to. Use `["*"]` for all pools.
        pool_ids: Vec<String>,
    },
    /// Unsubscribe from events for specific pools.
    Unsubscribe {
        /// Pool IDs to unsubscribe from.
        pool_ids: Vec<String>,
    },
    /// Execute a swap via WebSocket.
    Swap {
        /// Target pool ID.
        pool_id: String,
        /// Input token address.
        token_in: String,
        /// Swap specification (exact_in or exact_out).
        spec: serde_json::Value,
    },
    /// Get a swap quote (read-only).
    Quote {
        /// Target pool ID.
        pool_id: String,
        /// Input token address.
        token_in: String,
        /// Swap specification.
        spec: serde_json::Value,
    },
    /// Get full pool state.
    GetState {
        /// Target pool ID.
        pool_id: String,
    },
}
