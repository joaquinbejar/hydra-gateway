//! Liquidity operation handlers: add, remove, collect fees.

use axum::extract::{Path, State};
use axum::response::IntoResponse;
use axum::routing::post;
use axum::{Json, Router};
use chrono::Utc;
use hydra_amm::domain::{Amount, Liquidity, LiquidityChange};

use crate::api::dto::{
    AddLiquidityRequest, AddLiquidityResponse, RemoveLiquidityRequest, RemoveLiquidityResponse,
};
use crate::app_state::AppState;
use crate::domain::PoolId;
use crate::error::GatewayError;

/// `POST /pools/:id/liquidity/add` — Add liquidity to a pool.
async fn add_liquidity(
    State(state): State<AppState>,
    Path(id): Path<uuid::Uuid>,
    Json(req): Json<AddLiquidityRequest>,
) -> Result<impl IntoResponse, GatewayError> {
    let pool_id = PoolId::from_uuid(id);

    let amount_a: u128 = req
        .amount_a
        .parse()
        .map_err(|_| GatewayError::InvalidRequest(format!("invalid amount_a: {}", req.amount_a)))?;
    let amount_b: u128 = req
        .amount_b
        .parse()
        .map_err(|_| GatewayError::InvalidRequest(format!("invalid amount_b: {}", req.amount_b)))?;

    let change = LiquidityChange::add(Amount::new(amount_a), Amount::new(amount_b))?;
    let minted = state.pool_service.add_liquidity(pool_id, &change).await?;

    Ok(Json(AddLiquidityResponse {
        pool_id,
        amount_a_deposited: amount_a.to_string(),
        amount_b_deposited: amount_b.to_string(),
        liquidity_minted: minted.get().to_string(),
        executed_at: Utc::now(),
    }))
}

/// `POST /pools/:id/liquidity/remove` — Remove liquidity from a pool.
async fn remove_liquidity(
    State(state): State<AppState>,
    Path(id): Path<uuid::Uuid>,
    Json(req): Json<RemoveLiquidityRequest>,
) -> Result<impl IntoResponse, GatewayError> {
    let pool_id = PoolId::from_uuid(id);

    let liq_amount: u128 = req.liquidity_amount.parse().map_err(|_| {
        GatewayError::InvalidRequest(format!(
            "invalid liquidity_amount: {}",
            req.liquidity_amount
        ))
    })?;

    let change = LiquidityChange::remove(Liquidity::new(liq_amount))?;
    let returned = state
        .pool_service
        .remove_liquidity(pool_id, &change)
        .await?;

    Ok(Json(RemoveLiquidityResponse {
        pool_id,
        amount_returned: returned.get().to_string(),
        liquidity_burned: liq_amount.to_string(),
        executed_at: Utc::now(),
    }))
}

/// Liquidity routes.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/pools/{id}/liquidity/add", post(add_liquidity))
        .route("/pools/{id}/liquidity/remove", post(remove_liquidity))
}
