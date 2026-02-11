//! Liquidity operation DTOs.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::domain::PoolId;

/// Request body for `POST /pools/:id/liquidity/add`.
#[derive(Debug, Deserialize, ToSchema)]
pub struct AddLiquidityRequest {
    /// Amount of token A to deposit (string-encoded u128).
    pub amount_a: String,
    /// Amount of token B to deposit (string-encoded u128).
    pub amount_b: String,
    /// Maximum slippage tolerance (percentage as string, e.g. `"0.5"`).
    #[serde(default)]
    pub slippage_tolerance: Option<String>,
    /// Transaction deadline (ISO-8601).
    #[serde(default)]
    pub deadline: Option<DateTime<Utc>>,
}

/// Response body for `POST /pools/:id/liquidity/add`.
#[derive(Debug, Serialize, ToSchema)]
pub struct AddLiquidityResponse {
    /// Pool identifier.
    pub pool_id: PoolId,
    /// Token A amount deposited (string-encoded).
    pub amount_a_deposited: String,
    /// Token B amount deposited (string-encoded).
    pub amount_b_deposited: String,
    /// LP tokens or shares minted (string-encoded).
    pub liquidity_minted: String,
    /// Execution timestamp.
    pub executed_at: DateTime<Utc>,
}

/// Request body for `POST /pools/:id/liquidity/remove`.
#[derive(Debug, Deserialize, ToSchema)]
pub struct RemoveLiquidityRequest {
    /// Amount of LP tokens to burn (string-encoded u128).
    pub liquidity_amount: String,
    /// Minimum token A out for slippage protection.
    #[serde(default)]
    pub amount_a_min: Option<String>,
    /// Minimum token B out for slippage protection.
    #[serde(default)]
    pub amount_b_min: Option<String>,
    /// Transaction deadline (ISO-8601).
    #[serde(default)]
    pub deadline: Option<DateTime<Utc>>,
}

/// Response body for `POST /pools/:id/liquidity/remove`.
#[derive(Debug, Serialize, ToSchema)]
pub struct RemoveLiquidityResponse {
    /// Pool identifier.
    pub pool_id: PoolId,
    /// Tokens returned (string-encoded combined value).
    pub amount_returned: String,
    /// LP tokens burned (string-encoded).
    pub liquidity_burned: String,
    /// Execution timestamp.
    pub executed_at: DateTime<Utc>,
}

/// Request body for `POST /pools/:id/fees/collect`.
#[derive(Debug, Deserialize, ToSchema)]
pub struct CollectFeesRequest {
    /// Position tick range for CLMM pools.
    #[serde(default)]
    pub lower_tick: Option<i32>,
    /// Position upper tick for CLMM pools.
    #[serde(default)]
    pub upper_tick: Option<i32>,
}

/// Response body for `POST /pools/:id/fees/collect`.
#[derive(Debug, Serialize, ToSchema)]
pub struct CollectFeesResponse {
    /// Pool identifier.
    pub pool_id: PoolId,
    /// Fees collected (string-encoded).
    pub fees_collected: String,
    /// Collection timestamp.
    pub collected_at: DateTime<Utc>,
}
