//! REST API layer: route handlers, DTOs, and router composition.
//!
//! All endpoints are mounted under `/api/v1`.

pub mod dto;
pub mod handlers;

use axum::Router;
use utoipa::OpenApi;

use crate::app_state::AppState;

/// OpenAPI documentation for the hydra-gateway REST API.
#[derive(OpenApi)]
#[openapi(
    info(
        title = "hydra-gateway",
        version = "0.1.0",
        description = "REST API and WebSocket gateway for the hydra-amm universal AMM engine.",
        license(name = "MIT"),
        contact(name = "Joaquin Bejar", email = "jb@taunais.com"),
    ),
    tags(
        (name = "System", description = "Health check and service configuration"),
        (name = "Pools", description = "Pool CRUD operations"),
        (name = "Swaps", description = "Token swap execution and quoting"),
        (name = "Liquidity", description = "Liquidity provisioning and withdrawal"),
    ),
    paths(
        handlers::system::health_handler,
        handlers::system::pool_types_handler,
        handlers::pool::create_pool,
        handlers::pool::list_pools,
        handlers::pool::get_pool,
        handlers::pool::delete_pool,
        handlers::swap::execute_swap,
        handlers::swap::quote_swap,
        handlers::liquidity::add_liquidity,
        handlers::liquidity::remove_liquidity,
    ),
    components(schemas(
        crate::domain::PoolId,
        crate::error::ErrorResponse,
        crate::error::ErrorBody,
        dto::TokenDto,
        dto::PaginationParams,
        dto::PaginationMeta,
        dto::CreatePoolRequest,
        dto::CreatePoolResponse,
        dto::PoolDetailResponse,
        dto::PoolSummaryDto,
        dto::PoolListResponse,
        dto::SwapRequest,
        dto::SwapResponse,
        dto::QuoteResponse,
        dto::AddLiquidityRequest,
        dto::AddLiquidityResponse,
        dto::RemoveLiquidityRequest,
        dto::RemoveLiquidityResponse,
        dto::CollectFeesRequest,
        dto::CollectFeesResponse,
    ))
)]
#[derive(Debug)]
pub struct ApiDoc;

/// Builds the complete API router with all REST endpoints.
pub fn build_router() -> Router<AppState> {
    Router::new()
        .nest("/api/v1", handlers::routes())
        .merge(handlers::system::routes())
}
