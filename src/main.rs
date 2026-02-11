//! hydra-gateway server entry point.
//!
//! Starts the Axum HTTP server with REST and WebSocket endpoints.

use std::sync::Arc;

use axum::Router;
use axum::routing::get;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use tracing_subscriber::EnvFilter;

use hydra_gateway::api;
use hydra_gateway::app_state::AppState;
use hydra_gateway::config::GatewayConfig;
use hydra_gateway::domain::{EventBus, PoolRegistry};
use hydra_gateway::service::PoolService;
use hydra_gateway::ws::handler::ws_handler;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    // Load configuration
    let config = GatewayConfig::from_env()?;
    tracing::info!(addr = %config.listen_addr, "starting hydra-gateway");

    // Build domain layer
    let registry = Arc::new(PoolRegistry::new());
    let event_bus = EventBus::new(config.event_bus_capacity);

    // Build service layer
    let pool_service = Arc::new(PoolService::new(registry, event_bus.clone()));

    // Build application state
    let app_state = AppState {
        pool_service,
        event_bus,
    };

    // Build router
    let app = Router::new()
        .merge(api::build_router())
        .route("/ws", get(ws_handler))
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::permissive())
        .with_state(app_state);

    // Start server
    let listener = tokio::net::TcpListener::bind(config.listen_addr).await?;
    tracing::info!(addr = %config.listen_addr, "server listening");

    axum::serve(listener, app).await?;

    Ok(())
}
