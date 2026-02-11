//! WebSocket connection state machine.
//!
//! Handles the read/write loop for a single WebSocket connection,
//! dispatching incoming commands and forwarding filtered events.

use axum::extract::ws::{Message, WebSocket};
use futures_util::{SinkExt, StreamExt};
use tokio::sync::broadcast;

use super::messages::{WsMessage, WsMessageType};
use super::subscription::SubscriptionManager;
use crate::domain::{PoolEvent, PoolId};
use crate::service::PoolService;

/// Runs the read/write loop for a single WebSocket connection.
///
/// - Reads commands from the client and dispatches them.
/// - Forwards matching events from the [`broadcast::Receiver`] to the client.
pub async fn run_connection(
    socket: WebSocket,
    mut event_rx: broadcast::Receiver<PoolEvent>,
    _pool_service: std::sync::Arc<PoolService>,
) {
    let (mut ws_tx, mut ws_rx) = socket.split();
    let mut subs = SubscriptionManager::new();

    loop {
        tokio::select! {
            // Incoming message from client
            msg = ws_rx.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        let response = handle_text_message(&text, &mut subs);
                        if let Some(resp_json) = response
                            && ws_tx.send(Message::text(resp_json)).await.is_err() {
                                break;
                            }
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    _ => {}
                }
            }
            // Event from EventBus
            event = event_rx.recv() => {
                match event {
                    Ok(pool_event) => {
                        if subs.matches(pool_event.pool_id()) {
                            let msg = WsMessage {
                                id: uuid::Uuid::new_v4().to_string(),
                                msg_type: WsMessageType::Event,
                                timestamp: chrono::Utc::now(),
                                payload: serde_json::to_value(&pool_event).unwrap_or_default(),
                            };
                            let json = serde_json::to_string(&msg).unwrap_or_default();
                            if ws_tx.send(Message::text(json)).await.is_err() {
                                break;
                            }
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!(lagged = n, "ws client lagged behind event bus");
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
        }
    }

    tracing::debug!("ws connection closed");
}

/// Handles a text message from the client, returning an optional JSON response.
fn handle_text_message(text: &str, subs: &mut SubscriptionManager) -> Option<String> {
    let Ok(msg) = serde_json::from_str::<WsMessage>(text) else {
        let err = WsMessage {
            id: String::new(),
            msg_type: WsMessageType::Error,
            timestamp: chrono::Utc::now(),
            payload: serde_json::json!({
                "code": 400,
                "message": "malformed JSON"
            }),
        };
        return serde_json::to_string(&err).ok();
    };

    // Try to parse as a command with pool_ids for subscribe/unsubscribe
    if let Some(pool_ids) = msg.payload.get("pool_ids").and_then(|v| v.as_array()) {
        let command = msg
            .payload
            .get("command")
            .and_then(|v| v.as_str())
            .unwrap_or("subscribe");

        match command {
            "subscribe" => {
                let mut ids = Vec::new();
                let mut wildcard = false;
                for id_val in pool_ids {
                    if let Some(s) = id_val.as_str() {
                        if s == "*" {
                            wildcard = true;
                        } else if let Ok(uuid) = s.parse::<uuid::Uuid>() {
                            ids.push(PoolId::from_uuid(uuid));
                        }
                    }
                }
                subs.subscribe(&ids, wildcard);
                let response = WsMessage {
                    id: msg.id,
                    msg_type: WsMessageType::Response,
                    timestamp: chrono::Utc::now(),
                    payload: serde_json::json!({
                        "subscribed": ids.iter().map(|id| id.to_string()).collect::<Vec<_>>(),
                        "count": subs.count(),
                        "wildcard": subs.is_subscribed_all(),
                    }),
                };
                return serde_json::to_string(&response).ok();
            }
            "unsubscribe" => {
                let mut ids = Vec::new();
                for id_val in pool_ids {
                    if let Some(s) = id_val.as_str()
                        && let Ok(uuid) = s.parse::<uuid::Uuid>()
                    {
                        ids.push(PoolId::from_uuid(uuid));
                    }
                }
                subs.unsubscribe(&ids);
                let response = WsMessage {
                    id: msg.id,
                    msg_type: WsMessageType::Response,
                    timestamp: chrono::Utc::now(),
                    payload: serde_json::json!({
                        "unsubscribed": ids.iter().map(|id| id.to_string()).collect::<Vec<_>>(),
                        "remaining_count": subs.count(),
                    }),
                };
                return serde_json::to_string(&response).ok();
            }
            _ => {}
        }
    }

    // Unknown command
    let err = WsMessage {
        id: msg.id,
        msg_type: WsMessageType::Error,
        timestamp: chrono::Utc::now(),
        payload: serde_json::json!({
            "code": 404,
            "message": "unknown command"
        }),
    };
    serde_json::to_string(&err).ok()
}
