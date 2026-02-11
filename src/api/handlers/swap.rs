//! Swap and quote endpoint handlers.

use axum::extract::{Path, State};
use axum::response::IntoResponse;
use axum::routing::post;
use axum::{Json, Router};
use chrono::Utc;
use hydra_amm::domain::{Amount, SwapSpec, Token, TokenAddress};
use hydra_amm::traits::SwapPool;

use crate::api::dto::{QuoteResponse, SwapRequest, SwapResponse};
use crate::app_state::AppState;
use crate::domain::PoolId;
use crate::error::{ErrorResponse, GatewayError};

/// `POST /pools/:id/swap` — Execute a swap.
///
/// # Errors
///
/// Returns [`GatewayError`] on invalid parameters, missing pool, or insufficient liquidity.
#[utoipa::path(
    post,
    path = "/api/v1/pools/{id}/swap",
    tag = "Swaps",
    summary = "Execute a swap",
    description = "Executes a token swap on the specified pool. Supports exact-in and exact-out modes.",
    params(
        ("id" = uuid::Uuid, Path, description = "Pool UUID"),
    ),
    request_body = SwapRequest,
    responses(
        (status = 200, description = "Swap executed", body = SwapResponse),
        (status = 400, description = "Invalid swap parameters", body = ErrorResponse),
        (status = 404, description = "Pool not found", body = ErrorResponse),
        (status = 422, description = "Insufficient liquidity", body = ErrorResponse),
    )
)]
pub async fn execute_swap(
    State(state): State<AppState>,
    Path(id): Path<uuid::Uuid>,
    Json(req): Json<SwapRequest>,
) -> Result<impl IntoResponse, GatewayError> {
    let pool_id = PoolId::from_uuid(id);
    let (spec, token_in) = parse_swap_request(&state, pool_id, &req).await?;

    let command_id = uuid::Uuid::new_v4().to_string();

    // Capture price before
    let entry_lock = state.pool_service.registry().get(pool_id).await?;
    let entry = entry_lock.read().await;
    let pair = *entry.pool_box.token_pair();
    let base = pair.first();
    let quote_tok = pair.second();
    let price_before = entry
        .pool_box
        .spot_price(&base, &quote_tok)
        .map(|p| p.get())
        .unwrap_or(0.0);
    drop(entry);

    let result = state
        .pool_service
        .execute_swap(pool_id, spec, token_in, &command_id)
        .await?;

    // Capture price after
    let entry_lock = state.pool_service.registry().get(pool_id).await?;
    let entry = entry_lock.read().await;
    let price_after = entry
        .pool_box
        .spot_price(&base, &quote_tok)
        .map(|p| p.get())
        .unwrap_or(0.0);
    drop(entry);

    let price_impact_bps = if price_before == 0.0 {
        0
    } else {
        #[allow(clippy::cast_possible_truncation)]
        {
            ((price_after - price_before) / price_before * 10_000.0) as i32
        }
    };

    let effective_price = if result.amount_in().get() == 0 {
        "0".to_string()
    } else {
        format!(
            "{}",
            result.amount_out().get() as f64 / result.amount_in().get() as f64
        )
    };

    Ok(Json(SwapResponse {
        swap_id: command_id,
        pool_id,
        token_in: req.token_in,
        token_out: req.token_out,
        amount_in: result.amount_in().get().to_string(),
        amount_out: result.amount_out().get().to_string(),
        fee_charged: result.fee().get().to_string(),
        execution_price: effective_price,
        spot_price_before: format!("{price_before}"),
        spot_price_after: format!("{price_after}"),
        price_impact_bps,
        executed_at: Utc::now(),
    }))
}

/// `POST /pools/:id/quote` — Get swap quote (read-only).
///
/// # Errors
///
/// Returns [`GatewayError`] on invalid parameters or missing pool.
#[utoipa::path(
    post,
    path = "/api/v1/pools/{id}/quote",
    tag = "Swaps",
    summary = "Get swap quote",
    description = "Returns a price quote for a swap without executing it. The pool state is not modified.",
    params(
        ("id" = uuid::Uuid, Path, description = "Pool UUID"),
    ),
    request_body = SwapRequest,
    responses(
        (status = 200, description = "Quote computed", body = QuoteResponse),
        (status = 400, description = "Invalid swap parameters", body = ErrorResponse),
        (status = 404, description = "Pool not found", body = ErrorResponse),
    )
)]
pub async fn quote_swap(
    State(state): State<AppState>,
    Path(id): Path<uuid::Uuid>,
    Json(req): Json<SwapRequest>,
) -> Result<impl IntoResponse, GatewayError> {
    let pool_id = PoolId::from_uuid(id);
    let (spec, token_in) = parse_swap_request(&state, pool_id, &req).await?;

    // Get current spot price
    let entry_lock = state.pool_service.registry().get(pool_id).await?;
    let entry = entry_lock.read().await;
    let pair = *entry.pool_box.token_pair();
    let base = pair.first();
    let quote_tok = pair.second();
    let spot_price = entry
        .pool_box
        .spot_price(&base, &quote_tok)
        .map(|p| p.get())
        .unwrap_or(0.0);
    drop(entry);

    let result = state
        .pool_service
        .quote_swap(pool_id, spec, token_in)
        .await?;

    let effective_price = if result.amount_in().get() == 0 {
        "0".to_string()
    } else {
        format!(
            "{}",
            result.amount_out().get() as f64 / result.amount_in().get() as f64
        )
    };

    let price_after_quote = if spot_price == 0.0 {
        0.0
    } else {
        result.amount_out().get() as f64 / result.amount_in().get() as f64
    };

    let price_impact_bps = if spot_price == 0.0 {
        0
    } else {
        #[allow(clippy::cast_possible_truncation)]
        {
            ((price_after_quote - spot_price) / spot_price * 10_000.0) as i32
        }
    };

    Ok(Json(QuoteResponse {
        pool_id,
        token_in: req.token_in,
        token_out: req.token_out,
        amount_in: result.amount_in().get().to_string(),
        amount_out: result.amount_out().get().to_string(),
        fee_charged: result.fee().get().to_string(),
        execution_price: effective_price,
        spot_price: format!("{spot_price}"),
        price_impact_bps,
        quoted_at: Utc::now(),
    }))
}

/// Swap routes.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/pools/{id}/swap", post(execute_swap))
        .route("/pools/{id}/quote", post(quote_swap))
}

/// Parses a [`SwapRequest`] into a hydra-amm [`SwapSpec`] and input [`Token`].
async fn parse_swap_request(
    state: &AppState,
    pool_id: PoolId,
    req: &SwapRequest,
) -> Result<(SwapSpec, Token), GatewayError> {
    // Determine exact-in vs exact-out
    let spec = match (&req.amount_in, &req.amount_out) {
        (Some(amt_in), None) => {
            let amount: u128 = amt_in.parse().map_err(|_| {
                GatewayError::InvalidRequest(format!("invalid amount_in: {amt_in}"))
            })?;
            SwapSpec::exact_in(Amount::new(amount))?
        }
        (None, Some(amt_out)) => {
            let amount: u128 = amt_out.parse().map_err(|_| {
                GatewayError::InvalidRequest(format!("invalid amount_out: {amt_out}"))
            })?;
            SwapSpec::exact_out(Amount::new(amount))?
        }
        (Some(_), Some(_)) => {
            return Err(GatewayError::InvalidRequest(
                "specify either amount_in or amount_out, not both".to_string(),
            ));
        }
        (None, None) => {
            return Err(GatewayError::InvalidRequest(
                "must specify amount_in or amount_out".to_string(),
            ));
        }
    };

    // Resolve token_in from address string
    let entry_lock = state.pool_service.registry().get(pool_id).await?;
    let entry = entry_lock.read().await;
    let pair = *entry.pool_box.token_pair();
    let first = pair.first();
    let second = pair.second();
    drop(entry);

    // Match token_in address against the pool's token pair
    let mut addr_bytes_in = [0u8; 32];
    let in_bytes = req.token_in.as_bytes();
    let in_len = in_bytes.len().min(32);
    if let (Some(dst), Some(src)) = (addr_bytes_in.get_mut(..in_len), in_bytes.get(..in_len)) {
        dst.copy_from_slice(src);
    }
    let addr_in = TokenAddress::from_bytes(addr_bytes_in);

    let token_in = if first.address() == addr_in {
        first
    } else if second.address() == addr_in {
        second
    } else {
        return Err(GatewayError::InvalidRequest(format!(
            "token_in {} not found in pool",
            req.token_in
        )));
    };

    Ok((spec, token_in))
}
