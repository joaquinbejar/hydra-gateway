//! System endpoints: health check, pool types, admin.

use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::{Json, Router};
use chrono::Utc;
use serde::Serialize;
use utoipa::ToSchema;

use crate::app_state::AppState;

/// Health check response.
#[derive(Debug, Serialize, ToSchema)]
struct HealthResponse {
    status: String,
    timestamp: String,
    version: String,
}

/// `GET /health` — Service health status.
#[utoipa::path(
    get,
    path = "/health",
    tag = "System",
    summary = "Health check",
    description = "Returns service health status, version, and current timestamp.",
    responses(
        (status = 200, description = "Service is healthy", body = HealthResponse),
    )
)]
pub async fn health_handler() -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(HealthResponse {
            status: "healthy".to_string(),
            timestamp: Utc::now().to_rfc3339(),
            version: env!("CARGO_PKG_VERSION").to_string(),
        }),
    )
}

/// Supported pool type info.
#[derive(Debug, Serialize, ToSchema)]
struct PoolTypeInfo {
    pool_type: &'static str,
    description: &'static str,
    multi_token: bool,
    tick_based: bool,
}

/// `GET /config/pool-types` — List supported pool types.
#[utoipa::path(
    get,
    path = "/config/pool-types",
    tag = "System",
    summary = "List supported pool types",
    description = "Returns metadata for every AMM pool type the gateway can create.",
    responses(
        (status = 200, description = "Pool type catalog", body = Vec<PoolTypeInfo>),
    )
)]
pub async fn pool_types_handler() -> impl IntoResponse {
    let types = vec![
        PoolTypeInfo {
            pool_type: "constant_product",
            description: "Uniswap V2 style (x · y = k)",
            multi_token: false,
            tick_based: false,
        },
        PoolTypeInfo {
            pool_type: "clmm",
            description: "Concentrated Liquidity (Uniswap V3 style)",
            multi_token: false,
            tick_based: true,
        },
        PoolTypeInfo {
            pool_type: "hybrid",
            description: "Curve-style StableSwap with amplification",
            multi_token: false,
            tick_based: false,
        },
        PoolTypeInfo {
            pool_type: "weighted",
            description: "Balancer-style weighted multi-token pools",
            multi_token: true,
            tick_based: false,
        },
        PoolTypeInfo {
            pool_type: "dynamic",
            description: "DODO-style Proactive Market Maker (oracle-driven)",
            multi_token: false,
            tick_based: false,
        },
        PoolTypeInfo {
            pool_type: "orderbook",
            description: "Phoenix-style CLOB + AMM hybrid",
            multi_token: false,
            tick_based: false,
        },
    ];
    (StatusCode::OK, Json(types))
}

/// System routes mounted at the root level (not under /api/v1).
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/health", get(health_handler))
        .route("/config/pool-types", get(pool_types_handler))
}
