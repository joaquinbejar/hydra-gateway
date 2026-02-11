//! Pool entry combining hydra-amm pool with server-side metadata.

use chrono::{DateTime, Utc};
use hydra_amm::pools::PoolBox;
use serde::Serialize;

use super::PoolId;

/// Aggregate wrapping a hydra-amm [`PoolBox`] with gateway metadata.
///
/// Each pool in the registry is stored as a `PoolEntry`. The `pool_box`
/// field holds the live AMM state (reserves, positions, etc.) while the
/// remaining fields track operational metadata.
#[derive(Debug)]
pub struct PoolEntry {
    /// Unique pool identifier (immutable after creation).
    pub pool_id: PoolId,

    /// The hydra-amm pool instance. Updated on swap / liquidity operations.
    pub pool_box: PoolBox,

    /// Pool type discriminator string (e.g. `"constant_product"`).
    pub pool_type: String,

    /// ISO-8601 creation timestamp (immutable after creation).
    pub created_at: DateTime<Utc>,

    /// ISO-8601 timestamp of last state mutation.
    pub last_modified_at: DateTime<Utc>,

    /// Number of swaps executed on this pool.
    pub swap_count: u64,

    /// Cumulative swap volume in base token smallest units.
    pub total_volume: u128,

    /// Fee tier in basis points (immutable after creation).
    pub fee_bps: u32,
}

impl PoolEntry {
    /// Creates a new `PoolEntry` with the given pool and metadata.
    #[must_use]
    pub fn new(pool_id: PoolId, pool_box: PoolBox, pool_type: String, fee_bps: u32) -> Self {
        let now = Utc::now();
        Self {
            pool_id,
            pool_box,
            pool_type,
            created_at: now,
            last_modified_at: now,
            swap_count: 0,
            total_volume: 0,
            fee_bps,
        }
    }
}

/// Lightweight summary of a pool for list endpoints.
#[derive(Debug, Clone, Serialize)]
pub struct PoolSummary {
    /// Pool identifier.
    pub pool_id: PoolId,
    /// Pool type string.
    pub pool_type: String,
    /// Creation timestamp.
    pub created_at: DateTime<Utc>,
    /// Fee tier in basis points.
    pub fee_bps: u32,
    /// Number of swaps executed.
    pub swap_count: u64,
}

impl From<&PoolEntry> for PoolSummary {
    fn from(entry: &PoolEntry) -> Self {
        Self {
            pool_id: entry.pool_id,
            pool_type: entry.pool_type.clone(),
            created_at: entry.created_at,
            fee_bps: entry.fee_bps,
            swap_count: entry.swap_count,
        }
    }
}
