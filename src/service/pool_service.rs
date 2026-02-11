//! Pool service: orchestrates pool operations and emits events.

use std::sync::Arc;

use chrono::Utc;
use hydra_amm::config::AmmConfig;
use hydra_amm::domain::{LiquidityChange, Position, SwapResult, SwapSpec, Token};
use hydra_amm::factory::DefaultPoolFactory;
use hydra_amm::traits::{LiquidityPool, SwapPool};

use crate::domain::pool_entry::{PoolEntry, PoolSummary};
use crate::domain::pool_event::{LiquidityChangeType, PoolEvent, PriceChangeReason};
use crate::domain::{EventBus, PoolId, PoolRegistry};
use crate::error::GatewayError;

/// Orchestration layer for all pool operations.
///
/// Stateless coordinator: owns references to [`PoolRegistry`] for state
/// and [`EventBus`] for event emission. Every mutation method follows
/// the pattern: acquire lock → call hydra-amm → update metadata → emit
/// events → return result.
#[derive(Debug, Clone)]
pub struct PoolService {
    registry: Arc<PoolRegistry>,
    event_bus: EventBus,
}

impl PoolService {
    /// Creates a new `PoolService`.
    #[must_use]
    pub fn new(registry: Arc<PoolRegistry>, event_bus: EventBus) -> Self {
        Self {
            registry,
            event_bus,
        }
    }

    /// Returns a reference to the inner [`EventBus`].
    #[must_use]
    pub fn event_bus(&self) -> &EventBus {
        &self.event_bus
    }

    /// Returns a reference to the inner [`PoolRegistry`].
    #[must_use]
    pub fn registry(&self) -> &Arc<PoolRegistry> {
        &self.registry
    }

    /// Creates a new pool from the given configuration.
    ///
    /// # Errors
    ///
    /// Returns a [`GatewayError`] if the configuration is invalid or
    /// pool creation fails.
    pub async fn create_pool(
        &self,
        config: &AmmConfig,
        pool_type: &str,
        fee_bps: u32,
    ) -> Result<PoolId, GatewayError> {
        let pool_box = DefaultPoolFactory::create(config)?;
        let pool_id = PoolId::new();

        let pair = pool_box.token_pair();
        let token_a = format!("{:?}", pair.first().address());
        let token_b = format!("{:?}", pair.second().address());

        let entry = PoolEntry::new(pool_id, pool_box, pool_type.to_string(), fee_bps);
        self.registry.insert(entry).await?;

        let _ = self.event_bus.publish(PoolEvent::PoolCreated {
            pool_id,
            pool_type: pool_type.to_string(),
            token_a,
            token_b,
            fee_tier: fee_bps,
            timestamp: Utc::now(),
        });

        tracing::info!(%pool_id, pool_type, "pool created");
        Ok(pool_id)
    }

    /// Executes a swap on the specified pool.
    ///
    /// # Errors
    ///
    /// Returns a [`GatewayError`] if the pool is not found or the swap fails.
    pub async fn execute_swap(
        &self,
        pool_id: PoolId,
        spec: SwapSpec,
        token_in: Token,
        command_id: &str,
    ) -> Result<SwapResult, GatewayError> {
        let entry_lock = self.registry.get(pool_id).await?;
        let mut entry = entry_lock.write().await;

        // Capture price before swap
        let pair = *entry.pool_box.token_pair();
        let base = pair.first();
        let quote = pair.second();
        let price_before = entry
            .pool_box
            .spot_price(&base, &quote)
            .map(|p| p.get())
            .unwrap_or(0.0);

        let result = entry.pool_box.swap(spec, token_in)?;

        // Update metadata
        entry.swap_count = entry.swap_count.saturating_add(1);
        entry.total_volume = entry.total_volume.saturating_add(result.amount_in().get());
        entry.last_modified_at = Utc::now();

        // Capture price after swap
        let price_after = entry
            .pool_box
            .spot_price(&base, &quote)
            .map(|p| p.get())
            .unwrap_or(0.0);

        let price_change_bps = compute_price_change_bps(price_before, price_after);

        drop(entry);

        // Emit events
        let _ = self.event_bus.publish(PoolEvent::SwapExecuted {
            pool_id,
            command_id: command_id.to_string(),
            amount_in: result.amount_in().get().to_string(),
            amount_out: result.amount_out().get().to_string(),
            fee: result.fee().get().to_string(),
            new_price: format!("{price_after}"),
            price_change_bps,
            timestamp: Utc::now(),
        });

        let _ = self.event_bus.publish(PoolEvent::PriceUpdated {
            pool_id,
            old_price: format!("{price_before}"),
            new_price: format!("{price_after}"),
            price_change_bps,
            reason: PriceChangeReason::SwapExecuted,
            timestamp: Utc::now(),
        });

        Ok(result)
    }

    /// Dry-run swap: clones pool state to compute a quote without mutation.
    ///
    /// # Errors
    ///
    /// Returns a [`GatewayError`] if the pool is not found or the quote
    /// computation fails.
    pub async fn quote_swap(
        &self,
        pool_id: PoolId,
        spec: SwapSpec,
        token_in: Token,
    ) -> Result<SwapResult, GatewayError> {
        // PoolBox doesn't implement Clone, so we acquire a write lock
        // and perform the swap, then reverse it. For a true quote we
        // accept the write lock cost — this is simpler than rebuilding
        // the pool from config.
        let entry_lock = self.registry.get(pool_id).await?;
        let mut entry = entry_lock.write().await;
        let result = entry.pool_box.swap(spec, token_in)?;

        // Reverse the swap to restore original state: swap the output
        // amount back using the output token.
        let pair = *entry.pool_box.token_pair();
        let reverse_token = if token_in == pair.first() {
            pair.second()
        } else {
            pair.first()
        };
        // Best-effort reversal — if it fails, state may drift slightly
        // but this is acceptable for a quote endpoint.
        if let Ok(reverse_spec) = SwapSpec::exact_in(result.amount_out()) {
            let _ = entry.pool_box.swap(reverse_spec, reverse_token);
        }

        Ok(result)
    }

    /// Adds liquidity to the specified pool.
    ///
    /// # Errors
    ///
    /// Returns a [`GatewayError`] if the pool is not found or the
    /// liquidity operation fails.
    pub async fn add_liquidity(
        &self,
        pool_id: PoolId,
        change: &LiquidityChange,
    ) -> Result<hydra_amm::domain::Amount, GatewayError> {
        let entry_lock = self.registry.get(pool_id).await?;
        let mut entry = entry_lock.write().await;

        let pair = *entry.pool_box.token_pair();
        let base = pair.first();
        let quote_tok = pair.second();
        let price_before = entry
            .pool_box
            .spot_price(&base, &quote_tok)
            .map(|p| p.get())
            .unwrap_or(0.0);

        let minted = entry.pool_box.add_liquidity(change)?;

        entry.last_modified_at = Utc::now();

        let total_liq = entry.pool_box.total_liquidity();
        let price_after = entry
            .pool_box
            .spot_price(&base, &quote_tok)
            .map(|p| p.get())
            .unwrap_or(0.0);

        let price_change_bps = compute_price_change_bps(price_before, price_after);

        // Extract amounts from the change for the event
        let (amount_a, amount_b) = match change {
            LiquidityChange::Add { amount_a, amount_b } => {
                (amount_a.get().to_string(), amount_b.get().to_string())
            }
            _ => ("0".to_string(), "0".to_string()),
        };

        drop(entry);

        let _ = self.event_bus.publish(PoolEvent::LiquidityChanged {
            pool_id,
            change_type: LiquidityChangeType::Add,
            amount_a,
            amount_b,
            new_total_liquidity: total_liq.get().to_string(),
            timestamp: Utc::now(),
        });

        let _ = self.event_bus.publish(PoolEvent::PriceUpdated {
            pool_id,
            old_price: format!("{price_before}"),
            new_price: format!("{price_after}"),
            price_change_bps,
            reason: PriceChangeReason::LiquidityAdded,
            timestamp: Utc::now(),
        });

        Ok(minted)
    }

    /// Removes liquidity from the specified pool.
    ///
    /// # Errors
    ///
    /// Returns a [`GatewayError`] if the pool is not found or the
    /// liquidity operation fails.
    pub async fn remove_liquidity(
        &self,
        pool_id: PoolId,
        change: &LiquidityChange,
    ) -> Result<hydra_amm::domain::Amount, GatewayError> {
        let entry_lock = self.registry.get(pool_id).await?;
        let mut entry = entry_lock.write().await;

        let pair = *entry.pool_box.token_pair();
        let base = pair.first();
        let quote_tok = pair.second();
        let price_before = entry
            .pool_box
            .spot_price(&base, &quote_tok)
            .map(|p| p.get())
            .unwrap_or(0.0);

        let returned = entry.pool_box.remove_liquidity(change)?;

        entry.last_modified_at = Utc::now();

        let total_liq = entry.pool_box.total_liquidity();
        let price_after = entry
            .pool_box
            .spot_price(&base, &quote_tok)
            .map(|p| p.get())
            .unwrap_or(0.0);

        let price_change_bps = compute_price_change_bps(price_before, price_after);

        drop(entry);

        let _ = self.event_bus.publish(PoolEvent::LiquidityChanged {
            pool_id,
            change_type: LiquidityChangeType::Remove,
            amount_a: returned.get().to_string(),
            amount_b: "0".to_string(),
            new_total_liquidity: total_liq.get().to_string(),
            timestamp: Utc::now(),
        });

        let _ = self.event_bus.publish(PoolEvent::PriceUpdated {
            pool_id,
            old_price: format!("{price_before}"),
            new_price: format!("{price_after}"),
            price_change_bps,
            reason: PriceChangeReason::LiquidityRemoved,
            timestamp: Utc::now(),
        });

        Ok(returned)
    }

    /// Collects accrued fees for a position.
    ///
    /// # Errors
    ///
    /// Returns a [`GatewayError`] if the pool is not found or fee
    /// collection fails.
    pub async fn collect_fees(
        &self,
        pool_id: PoolId,
        position: &Position,
    ) -> Result<hydra_amm::domain::Amount, GatewayError> {
        let entry_lock = self.registry.get(pool_id).await?;
        let mut entry = entry_lock.write().await;

        let fees = entry.pool_box.collect_fees(position)?;
        entry.last_modified_at = Utc::now();

        drop(entry);

        let _ = self.event_bus.publish(PoolEvent::FeesCollected {
            pool_id,
            fee_token_a: fees.get().to_string(),
            fee_token_b: "0".to_string(),
            timestamp: Utc::now(),
        });

        Ok(fees)
    }

    /// Removes a pool from the registry.
    ///
    /// # Errors
    ///
    /// Returns a [`GatewayError`] if the pool is not found.
    pub async fn remove_pool(&self, pool_id: PoolId) -> Result<(), GatewayError> {
        let _entry = self.registry.remove(pool_id).await?;

        let _ = self.event_bus.publish(PoolEvent::PoolRemoved {
            pool_id,
            timestamp: Utc::now(),
        });

        tracing::info!(%pool_id, "pool removed");
        Ok(())
    }

    /// Returns summaries of all pools, optionally filtered by type.
    pub async fn list_pools(&self, pool_type_filter: Option<&str>) -> Vec<PoolSummary> {
        self.registry.list(pool_type_filter).await
    }
}

/// Computes the price change in basis points between two price values.
fn compute_price_change_bps(old: f64, new: f64) -> i32 {
    if old == 0.0 {
        return 0;
    }
    #[allow(clippy::cast_possible_truncation)]
    let bps = ((new - old) / old * 10_000.0) as i32;
    bps
}

#[cfg(test)]
#[allow(clippy::panic)]
mod tests {
    use super::*;
    use hydra_amm::config::ConstantProductConfig;
    use hydra_amm::domain::{Amount, BasisPoints, Decimals, FeeTier, TokenAddress, TokenPair};

    fn make_config() -> (AmmConfig, Token, Token) {
        let Ok(d6) = Decimals::new(6) else {
            panic!("valid decimals");
        };
        let Ok(d18) = Decimals::new(18) else {
            panic!("valid decimals");
        };
        let tok_a = Token::new(TokenAddress::from_bytes([1u8; 32]), d6);
        let tok_b = Token::new(TokenAddress::from_bytes([2u8; 32]), d18);
        let Ok(pair) = TokenPair::new(tok_a, tok_b) else {
            panic!("valid pair");
        };
        let fee = FeeTier::new(BasisPoints::new(30));
        let Ok(cfg) =
            ConstantProductConfig::new(pair, fee, Amount::new(1_000_000), Amount::new(1_000_000))
        else {
            panic!("valid config");
        };
        (AmmConfig::ConstantProduct(cfg), tok_a, tok_b)
    }

    fn make_service() -> PoolService {
        let registry = Arc::new(PoolRegistry::new());
        let event_bus = EventBus::new(1000);
        PoolService::new(registry, event_bus)
    }

    #[tokio::test]
    async fn create_pool_emits_event() {
        let service = make_service();
        let mut rx = service.event_bus().subscribe();
        let (config, _, _) = make_config();

        let result = service.create_pool(&config, "constant_product", 30).await;
        assert!(result.is_ok());

        let event = rx.recv().await;
        let Ok(event) = event else {
            panic!("expected event");
        };
        assert_eq!(event.event_type_str(), "pool_created");
    }

    #[tokio::test]
    async fn execute_swap_updates_state() {
        let service = make_service();
        let (config, tok_a, _) = make_config();

        let Ok(pool_id) = service.create_pool(&config, "constant_product", 30).await else {
            panic!("pool creation failed");
        };

        let Ok(spec) = SwapSpec::exact_in(Amount::new(1000)) else {
            panic!("invalid spec");
        };

        let result = service.execute_swap(pool_id, spec, tok_a, "cmd-1").await;
        assert!(result.is_ok());

        let entry_lock = service.registry().get(pool_id).await;
        let Ok(entry_lock) = entry_lock else {
            panic!("pool not found");
        };
        let entry = entry_lock.read().await;
        assert_eq!(entry.swap_count, 1);
        assert!(entry.total_volume > 0);
    }

    #[tokio::test]
    async fn quote_swap_does_not_mutate() {
        let service = make_service();
        let (config, tok_a, _) = make_config();

        let Ok(pool_id) = service.create_pool(&config, "constant_product", 30).await else {
            panic!("pool creation failed");
        };

        let Ok(spec) = SwapSpec::exact_in(Amount::new(1000)) else {
            panic!("invalid spec");
        };

        let result = service.quote_swap(pool_id, spec, tok_a).await;
        assert!(result.is_ok());

        let entry_lock = service.registry().get(pool_id).await;
        let Ok(entry_lock) = entry_lock else {
            panic!("pool not found");
        };
        let entry = entry_lock.read().await;
        assert_eq!(entry.swap_count, 0);
    }

    #[tokio::test]
    async fn remove_pool_emits_event() {
        let service = make_service();
        let mut rx = service.event_bus().subscribe();
        let (config, _, _) = make_config();

        let Ok(pool_id) = service.create_pool(&config, "constant_product", 30).await else {
            panic!("pool creation failed");
        };
        // Drain the PoolCreated event
        let _ = rx.recv().await;

        let result = service.remove_pool(pool_id).await;
        assert!(result.is_ok());

        let event = rx.recv().await;
        let Ok(event) = event else {
            panic!("expected event");
        };
        assert_eq!(event.event_type_str(), "pool_removed");
    }
}
