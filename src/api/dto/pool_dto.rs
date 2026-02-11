//! Pool-related DTOs for create, get, and list operations.

use std::collections::HashMap;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::common_dto::{PaginationMeta, TokenDto};
use crate::domain::PoolId;

/// Request body for `POST /pools`.
#[derive(Debug, Deserialize)]
pub struct CreatePoolRequest {
    /// Pool type discriminator.
    pub pool_type: String,
    /// Optional human-readable name (max 100 chars).
    #[serde(default)]
    pub name: Option<String>,
    /// Pool-type-specific configuration.
    pub config: serde_json::Value,
}

/// Response body for `POST /pools` (201 Created).
#[derive(Debug, Serialize)]
pub struct CreatePoolResponse {
    /// Unique pool identifier.
    pub pool_id: PoolId,
    /// Pool type echoed from request.
    pub pool_type: String,
    /// Pool name echoed from request.
    pub name: Option<String>,
    /// Server creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Pool status.
    pub status: String,
}

/// Single pool detail for `GET /pools/:id`.
#[derive(Debug, Serialize)]
pub struct PoolDetailResponse {
    /// Pool identifier.
    pub pool_id: PoolId,
    /// Pool type string.
    pub pool_type: String,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Last update timestamp.
    pub updated_at: DateTime<Utc>,
    /// Pool status.
    pub status: String,
    /// Token metadata.
    pub tokens: Vec<TokenDto>,
    /// Current reserves keyed by token symbol.
    pub reserves: HashMap<String, String>,
    /// Current spot price.
    pub current_price: Option<String>,
    /// Total liquidity.
    pub total_liquidity: String,
    /// Fee tier in basis points.
    pub fee_bps: u32,
    /// Number of swaps executed.
    pub swap_count: u64,
}

/// Pool summary for list responses.
#[derive(Debug, Serialize)]
pub struct PoolSummaryDto {
    /// Pool identifier.
    pub pool_id: PoolId,
    /// Pool type string.
    pub pool_type: String,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Fee tier in basis points.
    pub fee_bps: u32,
    /// Number of swaps.
    pub swap_count: u64,
}

/// Paginated list response for `GET /pools`.
#[derive(Debug, Serialize)]
pub struct PoolListResponse {
    /// Pool summaries.
    pub data: Vec<PoolSummaryDto>,
    /// Pagination metadata.
    pub pagination: PaginationMeta,
}
