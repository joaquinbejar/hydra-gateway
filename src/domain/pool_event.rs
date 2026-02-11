//! Domain events reflecting pool state mutations.
//!
//! Every state change emits a [`PoolEvent`] through the [`super::EventBus`].
//! Events are broadcast to WebSocket subscribers and optionally persisted
//! to the PostgreSQL event log.

use chrono::{DateTime, Utc};
use serde::Serialize;

use super::PoolId;

/// Reason why a price update occurred.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PriceChangeReason {
    /// Price changed due to a swap execution.
    SwapExecuted,
    /// Price changed due to liquidity being added.
    LiquidityAdded,
    /// Price changed due to liquidity being removed.
    LiquidityRemoved,
}

/// Type of liquidity change.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LiquidityChangeType {
    /// Liquidity was added to the pool.
    Add,
    /// Liquidity was removed from the pool.
    Remove,
}

/// Domain event emitted after every state mutation.
///
/// All `Decimal`-like amounts are stored as `String` to preserve u128
/// precision when serialized to JSON.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "event_type", rename_all = "snake_case")]
pub enum PoolEvent {
    /// Emitted when a new pool is created.
    PoolCreated {
        /// Pool identifier.
        pool_id: PoolId,
        /// Pool type string (e.g. `"constant_product"`).
        pool_type: String,
        /// First token address.
        token_a: String,
        /// Second token address.
        token_b: String,
        /// Fee tier in basis points.
        fee_tier: u32,
        /// Creation timestamp.
        timestamp: DateTime<Utc>,
    },

    /// Emitted when a pool is removed.
    PoolRemoved {
        /// Pool identifier.
        pool_id: PoolId,
        /// Removal timestamp.
        timestamp: DateTime<Utc>,
    },

    /// Emitted after a successful swap.
    SwapExecuted {
        /// Pool identifier.
        pool_id: PoolId,
        /// Client-provided command ID for correlation.
        command_id: String,
        /// Input amount (string-encoded u128).
        amount_in: String,
        /// Output amount (string-encoded u128).
        amount_out: String,
        /// Fee charged (string-encoded u128).
        fee: String,
        /// New spot price after swap.
        new_price: String,
        /// Price change in basis points.
        price_change_bps: i32,
        /// Execution timestamp.
        timestamp: DateTime<Utc>,
    },

    /// Emitted after liquidity is added or removed.
    LiquidityChanged {
        /// Pool identifier.
        pool_id: PoolId,
        /// Whether liquidity was added or removed.
        change_type: LiquidityChangeType,
        /// Amount of token A involved.
        amount_a: String,
        /// Amount of token B involved.
        amount_b: String,
        /// New total liquidity after the change.
        new_total_liquidity: String,
        /// Timestamp of the change.
        timestamp: DateTime<Utc>,
    },

    /// Emitted after fees are collected from a position.
    FeesCollected {
        /// Pool identifier.
        pool_id: PoolId,
        /// Fees collected in token A.
        fee_token_a: String,
        /// Fees collected in token B.
        fee_token_b: String,
        /// Collection timestamp.
        timestamp: DateTime<Utc>,
    },

    /// Emitted after any operation that modifies the pool price.
    PriceUpdated {
        /// Pool identifier.
        pool_id: PoolId,
        /// Spot price before the operation.
        old_price: String,
        /// Spot price after the operation.
        new_price: String,
        /// Price change in basis points.
        price_change_bps: i32,
        /// Why the price changed.
        reason: PriceChangeReason,
        /// Timestamp of the price update.
        timestamp: DateTime<Utc>,
    },
}

impl PoolEvent {
    /// Returns the pool ID associated with this event.
    #[must_use]
    pub fn pool_id(&self) -> PoolId {
        match self {
            Self::PoolCreated { pool_id, .. }
            | Self::PoolRemoved { pool_id, .. }
            | Self::SwapExecuted { pool_id, .. }
            | Self::LiquidityChanged { pool_id, .. }
            | Self::FeesCollected { pool_id, .. }
            | Self::PriceUpdated { pool_id, .. } => *pool_id,
        }
    }

    /// Returns the event type as a static string slice.
    #[must_use]
    pub const fn event_type_str(&self) -> &'static str {
        match self {
            Self::PoolCreated { .. } => "pool_created",
            Self::PoolRemoved { .. } => "pool_removed",
            Self::SwapExecuted { .. } => "swap_executed",
            Self::LiquidityChanged { .. } => "liquidity_changed",
            Self::FeesCollected { .. } => "fees_collected",
            Self::PriceUpdated { .. } => "price_updated",
        }
    }
}

#[cfg(test)]
#[allow(clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn pool_created_event_type() {
        let event = PoolEvent::PoolCreated {
            pool_id: PoolId::new(),
            pool_type: "constant_product".to_string(),
            token_a: "0xaaa".to_string(),
            token_b: "0xbbb".to_string(),
            fee_tier: 30,
            timestamp: Utc::now(),
        };
        assert_eq!(event.event_type_str(), "pool_created");
    }

    #[test]
    fn swap_executed_serializes() {
        let event = PoolEvent::SwapExecuted {
            pool_id: PoolId::new(),
            command_id: "cmd-1".to_string(),
            amount_in: "1000".to_string(),
            amount_out: "990".to_string(),
            fee: "3".to_string(),
            new_price: "0.99".to_string(),
            price_change_bps: -10,
            timestamp: Utc::now(),
        };
        let json = serde_json::to_string(&event);
        assert!(json.is_ok());
        let json_str = json.unwrap_or_default();
        assert!(json_str.contains("swap_executed"));
        assert!(json_str.contains("1000"));
    }

    #[test]
    fn pool_id_accessor() {
        let id = PoolId::new();
        let event = PoolEvent::PoolRemoved {
            pool_id: id,
            timestamp: Utc::now(),
        };
        assert_eq!(event.pool_id(), id);
    }
}
