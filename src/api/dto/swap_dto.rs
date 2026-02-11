//! Swap and quote DTOs.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::domain::PoolId;

/// Request body for `POST /pools/:id/swap` and `POST /pools/:id/quote`.
#[derive(Debug, Deserialize, ToSchema)]
pub struct SwapRequest {
    /// Address of the input token.
    pub token_in: String,
    /// Address of the output token.
    pub token_out: String,
    /// Exact input amount (string-encoded u128). Mutually exclusive with `amount_out`.
    #[serde(default)]
    pub amount_in: Option<String>,
    /// Exact output amount (string-encoded u128). Mutually exclusive with `amount_in`.
    #[serde(default)]
    pub amount_out: Option<String>,
    /// Minimum output for slippage protection (string-encoded u128).
    #[serde(default)]
    pub min_amount_out: Option<String>,
    /// Maximum input for slippage protection on exact-out swaps.
    #[serde(default)]
    pub max_amount_in: Option<String>,
    /// Transaction deadline (ISO-8601).
    #[serde(default)]
    pub deadline: Option<DateTime<Utc>>,
}

/// Response body for `POST /pools/:id/swap`.
#[derive(Debug, Serialize, ToSchema)]
pub struct SwapResponse {
    /// Unique swap identifier.
    pub swap_id: String,
    /// Pool where the swap occurred.
    pub pool_id: PoolId,
    /// Input token address.
    pub token_in: String,
    /// Output token address.
    pub token_out: String,
    /// Actual input amount (string-encoded).
    pub amount_in: String,
    /// Actual output amount (string-encoded).
    pub amount_out: String,
    /// Fee charged (string-encoded).
    pub fee_charged: String,
    /// Effective execution price.
    pub execution_price: String,
    /// Spot price before swap.
    pub spot_price_before: String,
    /// Spot price after swap.
    pub spot_price_after: String,
    /// Price impact in basis points.
    pub price_impact_bps: i32,
    /// Execution timestamp.
    pub executed_at: DateTime<Utc>,
}

/// Response body for `POST /pools/:id/quote`.
#[derive(Debug, Serialize, ToSchema)]
pub struct QuoteResponse {
    /// Pool identifier.
    pub pool_id: PoolId,
    /// Input token address.
    pub token_in: String,
    /// Output token address.
    pub token_out: String,
    /// Input amount for quote (string-encoded).
    pub amount_in: String,
    /// Quoted output amount (string-encoded).
    pub amount_out: String,
    /// Fee amount (string-encoded).
    pub fee_charged: String,
    /// Effective price.
    pub execution_price: String,
    /// Current spot price.
    pub spot_price: String,
    /// Estimated price impact in basis points.
    pub price_impact_bps: i32,
    /// Quote timestamp.
    pub quoted_at: DateTime<Utc>,
}
