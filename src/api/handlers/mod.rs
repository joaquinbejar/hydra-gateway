//! REST endpoint handlers organized by resource.

pub mod liquidity;
pub mod pool;
pub mod swap;
pub mod system;

use axum::Router;

use crate::app_state::AppState;

/// Composes all resource routes under `/api/v1`.
pub fn routes() -> Router<AppState> {
    Router::new()
        .merge(pool::routes())
        .merge(swap::routes())
        .merge(liquidity::routes())
}
