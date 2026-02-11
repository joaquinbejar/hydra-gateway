//! Axum WebSocket upgrade handler.

use axum::extract::State;
use axum::extract::ws::WebSocketUpgrade;
use axum::response::IntoResponse;

use super::connection::run_connection;
use crate::app_state::AppState;

/// `GET /ws` — Upgrade HTTP connection to WebSocket.
pub async fn ws_handler(ws: WebSocketUpgrade, State(state): State<AppState>) -> impl IntoResponse {
    let event_rx = state.event_bus.subscribe();
    let pool_service = std::sync::Arc::clone(&state.pool_service);

    ws.on_upgrade(move |socket| run_connection(socket, event_rx, pool_service))
}
