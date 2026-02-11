//! WebSocket layer: connection handling, message routing, subscriptions.
//!
//! The WebSocket endpoint at `/ws` provides bidirectional communication
//! for real-time event subscriptions and command execution.

pub mod connection;
pub mod handler;
pub mod messages;
pub mod subscription;
