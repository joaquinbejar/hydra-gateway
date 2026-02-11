//! Shared application state injected into all Axum handlers.

use std::sync::Arc;

use crate::domain::EventBus;
use crate::service::PoolService;

/// Shared application state available to all handlers via Axum's
/// `State` extractor.
#[derive(Debug, Clone)]
pub struct AppState {
    /// Pool service for all business logic.
    pub pool_service: Arc<PoolService>,
    /// Event bus for WebSocket subscriptions.
    pub event_bus: EventBus,
}
