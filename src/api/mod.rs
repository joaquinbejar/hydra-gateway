//! REST API layer: route handlers, DTOs, and router composition.
//!
//! All endpoints are mounted under `/api/v1`.

pub mod dto;
pub mod handlers;

use axum::Router;

use crate::app_state::AppState;

/// Builds the complete API router with all REST endpoints.
pub fn build_router() -> Router<AppState> {
    Router::new()
        .nest("/api/v1", handlers::routes())
        .merge(handlers::system::routes())
}
