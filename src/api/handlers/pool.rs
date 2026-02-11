//! Pool CRUD handlers: create, list, get, delete.

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::routing::{get, post};
use axum::{Json, Router};
use chrono::Utc;
use hydra_amm::config::{
    AmmConfig, ClmmConfig, ConstantProductConfig, DynamicConfig, HybridConfig, OrderBookConfig,
    WeightedConfig,
};
use hydra_amm::domain::{
    Amount, BasisPoints, Decimals, FeeTier, Position, Price, Tick, Token, TokenAddress, TokenPair,
};

use crate::api::dto::{
    CreatePoolRequest, CreatePoolResponse, PaginationMeta, PaginationParams, PoolListResponse,
    PoolSummaryDto,
};
use crate::app_state::AppState;
use crate::error::{ErrorResponse, GatewayError};

/// `POST /pools` — Create a new AMM pool.
///
/// # Errors
///
/// Returns [`GatewayError`] on invalid config or unsupported pool type.
#[utoipa::path(
    post,
    path = "/api/v1/pools",
    tag = "Pools",
    summary = "Create a new AMM pool",
    description = "Creates a pool of the specified type with the given configuration. The `pool_type` field selects the AMM variant and `config` holds type-specific parameters.",
    request_body = CreatePoolRequest,
    responses(
        (status = 201, description = "Pool created successfully", body = CreatePoolResponse),
        (status = 400, description = "Invalid request or pool type", body = ErrorResponse),
    )
)]
pub async fn create_pool(
    State(state): State<AppState>,
    Json(req): Json<CreatePoolRequest>,
) -> Result<impl IntoResponse, GatewayError> {
    let (config, fee_bps) = parse_pool_config(&req)?;

    let pool_id = state
        .pool_service
        .create_pool(&config, &req.pool_type, fee_bps)
        .await?;

    let response = CreatePoolResponse {
        pool_id,
        pool_type: req.pool_type,
        name: req.name,
        created_at: Utc::now(),
        status: "active".to_string(),
    };

    Ok((StatusCode::CREATED, Json(response)))
}

/// `GET /pools` — List all pools with pagination and optional type filter.
///
/// # Errors
///
/// Returns [`GatewayError`] on internal failures.
#[utoipa::path(
    get,
    path = "/api/v1/pools",
    tag = "Pools",
    summary = "List pools",
    description = "Returns a paginated list of all pools, optionally filtered by type.",
    params(PaginationParams),
    responses(
        (status = 200, description = "Paginated pool list", body = PoolListResponse),
    )
)]
pub async fn list_pools(
    State(state): State<AppState>,
    Query(params): Query<PaginationParams>,
) -> Result<impl IntoResponse, GatewayError> {
    let params = params.clamped();
    let summaries = state.pool_service.list_pools(None).await;

    let total = summaries.len() as u32;
    let per_page = params.per_page;
    let page = params.page;
    let total_pages = if total == 0 {
        0
    } else {
        total.div_ceil(per_page)
    };

    let start = ((page - 1) * per_page) as usize;
    let data: Vec<PoolSummaryDto> = summaries
        .into_iter()
        .skip(start)
        .take(per_page as usize)
        .map(|s| PoolSummaryDto {
            pool_id: s.pool_id,
            pool_type: s.pool_type,
            created_at: s.created_at,
            fee_bps: s.fee_bps,
            swap_count: s.swap_count,
        })
        .collect();

    Ok(Json(PoolListResponse {
        data,
        pagination: PaginationMeta {
            page,
            per_page,
            total,
            total_pages,
        },
    }))
}

/// `GET /pools/:id` — Get pool details.
///
/// # Errors
///
/// Returns [`GatewayError::PoolNotFound`] if the pool does not exist.
#[utoipa::path(
    get,
    path = "/api/v1/pools/{id}",
    tag = "Pools",
    summary = "Get pool details",
    description = "Returns full details for a single pool including reserves, prices, and metadata.",
    params(
        ("id" = uuid::Uuid, Path, description = "Pool UUID"),
    ),
    responses(
        (status = 200, description = "Pool details", body = serde_json::Value),
        (status = 404, description = "Pool not found", body = ErrorResponse),
    )
)]
pub async fn get_pool(
    State(state): State<AppState>,
    Path(id): Path<uuid::Uuid>,
) -> Result<impl IntoResponse, GatewayError> {
    let pool_id = crate::domain::PoolId::from_uuid(id);
    let entry_lock = state.pool_service.registry().get(pool_id).await?;
    let entry = entry_lock.read().await;

    let response = serde_json::json!({
        "pool_id": entry.pool_id,
        "pool_type": entry.pool_type,
        "created_at": entry.created_at.to_rfc3339(),
        "updated_at": entry.last_modified_at.to_rfc3339(),
        "status": "active",
        "fee_bps": entry.fee_bps,
        "swap_count": entry.swap_count,
        "total_volume": entry.total_volume.to_string(),
    });

    Ok(Json(response))
}

/// `DELETE /pools/:id` — Remove a pool.
///
/// # Errors
///
/// Returns [`GatewayError::PoolNotFound`] if the pool does not exist.
#[utoipa::path(
    delete,
    path = "/api/v1/pools/{id}",
    tag = "Pools",
    summary = "Delete a pool",
    description = "Removes a pool and emits a PoolRemoved event.",
    params(
        ("id" = uuid::Uuid, Path, description = "Pool UUID"),
    ),
    responses(
        (status = 204, description = "Pool deleted"),
        (status = 404, description = "Pool not found", body = ErrorResponse),
    )
)]
pub async fn delete_pool(
    State(state): State<AppState>,
    Path(id): Path<uuid::Uuid>,
) -> Result<impl IntoResponse, GatewayError> {
    let pool_id = crate::domain::PoolId::from_uuid(id);
    state.pool_service.remove_pool(pool_id).await?;
    Ok(StatusCode::NO_CONTENT)
}

/// Pool management routes.
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/pools", post(create_pool).get(list_pools))
        .route("/pools/{id}", get(get_pool).delete(delete_pool))
}

// ── Config Parsing Helpers ──────────────────────────────────────────────

/// Parses the `CreatePoolRequest` JSON config into an `AmmConfig`.
///
/// # Errors
///
/// Returns a [`GatewayError`] on invalid or unsupported configuration.
fn parse_pool_config(req: &CreatePoolRequest) -> Result<(AmmConfig, u32), GatewayError> {
    match req.pool_type.as_str() {
        "constant_product" => parse_constant_product(&req.config),
        "clmm" => parse_clmm(&req.config),
        "hybrid" => parse_hybrid(&req.config),
        "weighted" => parse_weighted(&req.config),
        "dynamic" => parse_dynamic(&req.config),
        "orderbook" => parse_orderbook(&req.config),
        other => Err(GatewayError::InvalidPoolType(other.to_string())),
    }
}

fn parse_token(val: &serde_json::Value) -> Result<Token, GatewayError> {
    let address = val
        .get("address")
        .and_then(|v| v.as_str())
        .ok_or_else(|| GatewayError::InvalidRequest("missing token address".to_string()))?;

    let decimals = val
        .get("decimals")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| GatewayError::InvalidRequest("missing token decimals".to_string()))?;

    let mut bytes = [0u8; 32];
    let addr_bytes = address.as_bytes();
    let len = addr_bytes.len().min(32);
    if let (Some(dst), Some(src)) = (bytes.get_mut(..len), addr_bytes.get(..len)) {
        dst.copy_from_slice(src);
    }

    let decimals = Decimals::new(decimals as u8)
        .map_err(|e| GatewayError::InvalidRequest(format!("invalid decimals: {e}")))?;

    Ok(Token::new(TokenAddress::from_bytes(bytes), decimals))
}

fn parse_fee_bps(config: &serde_json::Value) -> Result<(FeeTier, u32), GatewayError> {
    let bps = config
        .get("fee_bps")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| GatewayError::InvalidRequest("missing fee_bps".to_string()))?;
    let bps_u32 = bps as u32;
    Ok((FeeTier::new(BasisPoints::new(bps_u32)), bps_u32))
}

fn parse_amount_str(val: &serde_json::Value, field: &str) -> Result<Amount, GatewayError> {
    let s = val
        .get(field)
        .and_then(|v| v.as_str().or_else(|| v.as_u64().map(|_| "")))
        .ok_or_else(|| GatewayError::InvalidRequest(format!("missing {field}")))?;

    // Handle both string and number formats
    let num: u128 = if s.is_empty() {
        val.get(field)
            .and_then(|v| v.as_u64())
            .map(u128::from)
            .ok_or_else(|| GatewayError::InvalidRequest(format!("invalid {field}")))?
    } else {
        s.parse()
            .map_err(|_| GatewayError::InvalidRequest(format!("invalid {field}: {s}")))?
    };

    Ok(Amount::new(num))
}

fn parse_constant_product(config: &serde_json::Value) -> Result<(AmmConfig, u32), GatewayError> {
    let token_a = parse_token(
        config
            .get("token_a")
            .ok_or_else(|| GatewayError::InvalidRequest("missing token_a".to_string()))?,
    )?;
    let token_b = parse_token(
        config
            .get("token_b")
            .ok_or_else(|| GatewayError::InvalidRequest("missing token_b".to_string()))?,
    )?;
    let (fee, fee_bps) = parse_fee_bps(config)?;
    let reserve_a = parse_amount_str(config, "reserve_a")?;
    let reserve_b = parse_amount_str(config, "reserve_b")?;

    let pair = TokenPair::new(token_a, token_b)?;
    let cfg = ConstantProductConfig::new(pair, fee, reserve_a, reserve_b)?;
    Ok((AmmConfig::ConstantProduct(cfg), fee_bps))
}

fn parse_clmm(config: &serde_json::Value) -> Result<(AmmConfig, u32), GatewayError> {
    let token_a = parse_token(
        config
            .get("token_a")
            .ok_or_else(|| GatewayError::InvalidRequest("missing token_a".to_string()))?,
    )?;
    let token_b = parse_token(
        config
            .get("token_b")
            .ok_or_else(|| GatewayError::InvalidRequest("missing token_b".to_string()))?,
    )?;
    let (fee, fee_bps) = parse_fee_bps(config)?;

    let tick_spacing = config
        .get("tick_spacing")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| GatewayError::InvalidRequest("missing tick_spacing".to_string()))?
        as u32;

    let current_tick_val = config
        .get("current_tick")
        .and_then(|v| v.as_i64())
        .ok_or_else(|| GatewayError::InvalidRequest("missing current_tick".to_string()))?
        as i32;

    let current_tick = Tick::new(current_tick_val)?;
    let pair = TokenPair::new(token_a, token_b)?;

    // Parse optional positions
    let positions = if let Some(pos_arr) = config.get("positions").and_then(|v| v.as_array()) {
        let mut result = Vec::with_capacity(pos_arr.len());
        for p in pos_arr {
            let lower = p
                .get("lower_tick")
                .and_then(|v| v.as_i64())
                .ok_or_else(|| {
                    GatewayError::InvalidRequest("missing position lower_tick".to_string())
                })? as i32;
            let upper = p
                .get("upper_tick")
                .and_then(|v| v.as_i64())
                .ok_or_else(|| {
                    GatewayError::InvalidRequest("missing position upper_tick".to_string())
                })? as i32;
            let liq = p
                .get("liquidity")
                .and_then(|v| v.as_str().or_else(|| v.as_u64().map(|_| "")))
                .ok_or_else(|| {
                    GatewayError::InvalidRequest("missing position liquidity".to_string())
                })?;
            let liq_val: u128 = if liq.is_empty() {
                p.get("liquidity")
                    .and_then(|v| v.as_u64())
                    .map(u128::from)
                    .ok_or_else(|| {
                        GatewayError::InvalidRequest("invalid position liquidity".to_string())
                    })?
            } else {
                liq.parse().map_err(|_| {
                    GatewayError::InvalidRequest("invalid position liquidity".to_string())
                })?
            };
            let pos = Position::new(
                Tick::new(lower)?,
                Tick::new(upper)?,
                hydra_amm::domain::Liquidity::new(liq_val),
            )?;
            result.push(pos);
        }
        result
    } else {
        vec![]
    };

    let cfg = ClmmConfig::new(pair, fee, tick_spacing, current_tick, positions)?;
    Ok((AmmConfig::Clmm(cfg), fee_bps))
}

fn parse_hybrid(config: &serde_json::Value) -> Result<(AmmConfig, u32), GatewayError> {
    let token_a = parse_token(
        config
            .get("token_a")
            .ok_or_else(|| GatewayError::InvalidRequest("missing token_a".to_string()))?,
    )?;
    let token_b = parse_token(
        config
            .get("token_b")
            .ok_or_else(|| GatewayError::InvalidRequest("missing token_b".to_string()))?,
    )?;
    let (fee, fee_bps) = parse_fee_bps(config)?;
    let amplification = config
        .get("amplification")
        .and_then(|v| v.as_u64())
        .ok_or_else(|| GatewayError::InvalidRequest("missing amplification".to_string()))?
        as u32;
    let reserve_a = parse_amount_str(config, "reserve_a")?;
    let reserve_b = parse_amount_str(config, "reserve_b")?;

    let pair = TokenPair::new(token_a, token_b)?;
    let cfg = HybridConfig::new(pair, fee, amplification, reserve_a, reserve_b)?;
    Ok((AmmConfig::Hybrid(cfg), fee_bps))
}

fn parse_weighted(config: &serde_json::Value) -> Result<(AmmConfig, u32), GatewayError> {
    let (fee, fee_bps) = parse_fee_bps(config)?;

    let tokens_arr = config
        .get("tokens")
        .and_then(|v| v.as_array())
        .ok_or_else(|| GatewayError::InvalidRequest("missing tokens array".to_string()))?;

    let mut tokens = Vec::with_capacity(tokens_arr.len());
    let mut weights = Vec::with_capacity(tokens_arr.len());
    for t in tokens_arr {
        tokens.push(parse_token(t)?);
        let w = t
            .get("weight")
            .and_then(|v| v.as_u64())
            .ok_or_else(|| GatewayError::InvalidRequest("missing token weight".to_string()))?
            as u32;
        weights.push(BasisPoints::new(w));
    }

    let reserves_arr = config
        .get("reserves")
        .and_then(|v| v.as_array())
        .ok_or_else(|| GatewayError::InvalidRequest("missing reserves array".to_string()))?;

    let mut balances = Vec::with_capacity(reserves_arr.len());
    for r in reserves_arr {
        let s = r
            .as_str()
            .ok_or_else(|| GatewayError::InvalidRequest("reserve must be string".to_string()))?;
        let val: u128 = s
            .parse()
            .map_err(|_| GatewayError::InvalidRequest(format!("invalid reserve: {s}")))?;
        balances.push(Amount::new(val));
    }

    let cfg = WeightedConfig::new(tokens, weights, fee, balances)?;
    Ok((AmmConfig::Weighted(cfg), fee_bps))
}

fn parse_dynamic(config: &serde_json::Value) -> Result<(AmmConfig, u32), GatewayError> {
    let token_a = parse_token(
        config
            .get("token_a")
            .ok_or_else(|| GatewayError::InvalidRequest("missing token_a".to_string()))?,
    )?;
    let token_b = parse_token(
        config
            .get("token_b")
            .ok_or_else(|| GatewayError::InvalidRequest("missing token_b".to_string()))?,
    )?;
    let (fee, fee_bps) = parse_fee_bps(config)?;

    let oracle_price_val = config
        .get("oracle_price")
        .and_then(|v| {
            v.as_f64()
                .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
        })
        .ok_or_else(|| GatewayError::InvalidRequest("missing oracle_price".to_string()))?;
    let oracle_price = Price::new(oracle_price_val)?;

    let slippage_coefficient = config
        .get("slippage_coefficient")
        .and_then(|v| {
            v.as_f64()
                .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
        })
        .ok_or_else(|| GatewayError::InvalidRequest("missing slippage_coefficient".to_string()))?;

    let reserve_a = parse_amount_str(config, "reserve_a")?;
    let reserve_b = parse_amount_str(config, "reserve_b")?;

    let pair = TokenPair::new(token_a, token_b)?;
    let cfg = DynamicConfig::new(
        pair,
        fee,
        oracle_price,
        slippage_coefficient,
        reserve_a,
        reserve_b,
    )?;
    Ok((AmmConfig::Dynamic(cfg), fee_bps))
}

fn parse_orderbook(config: &serde_json::Value) -> Result<(AmmConfig, u32), GatewayError> {
    let token_a = parse_token(
        config
            .get("token_a")
            .ok_or_else(|| GatewayError::InvalidRequest("missing token_a".to_string()))?,
    )?;
    let token_b = parse_token(
        config
            .get("token_b")
            .ok_or_else(|| GatewayError::InvalidRequest("missing token_b".to_string()))?,
    )?;
    let (fee, fee_bps) = parse_fee_bps(config)?;
    let tick_size = parse_amount_str(config, "tick_size")?;
    let lot_size = parse_amount_str(config, "lot_size")?;

    let pair = TokenPair::new(token_a, token_b)?;
    let cfg = OrderBookConfig::new(pair, fee, tick_size, lot_size)?;
    Ok((AmmConfig::OrderBook(cfg), fee_bps))
}
