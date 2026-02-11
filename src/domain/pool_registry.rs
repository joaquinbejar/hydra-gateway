//! Concurrent pool storage with per-pool fine-grained locking.
//!
//! [`PoolRegistry`] stores all active pools in a `HashMap` where each
//! entry is individually protected by a [`tokio::sync::RwLock`]. This
//! allows concurrent reads on the same pool and concurrent writes on
//! different pools.

use std::collections::HashMap;
use std::sync::Arc;

use tokio::sync::RwLock;

use super::PoolId;
use super::pool_entry::{PoolEntry, PoolSummary};
use crate::error::GatewayError;

/// Central store for all active AMM pools.
///
/// Uses a `RwLock<HashMap<...>>` for the outer map and per-entry
/// `Arc<RwLock<PoolEntry>>` for fine-grained per-pool locking.
///
/// # Concurrency
///
/// - Multiple threads may read the same pool concurrently.
/// - Writes to different pools are concurrent.
/// - Writes to the same pool are serialized.
#[derive(Debug)]
pub struct PoolRegistry {
    pools: RwLock<HashMap<PoolId, Arc<RwLock<PoolEntry>>>>,
}

impl PoolRegistry {
    /// Creates an empty registry.
    #[must_use]
    pub fn new() -> Self {
        Self {
            pools: RwLock::new(HashMap::new()),
        }
    }

    /// Inserts a new pool entry into the registry.
    ///
    /// # Errors
    ///
    /// Returns [`GatewayError::InvalidRequest`] if a pool with the same
    /// ID already exists (should never happen with UUID v4).
    pub async fn insert(&self, entry: PoolEntry) -> Result<PoolId, GatewayError> {
        let pool_id = entry.pool_id;
        let mut map = self.pools.write().await;
        if map.contains_key(&pool_id) {
            return Err(GatewayError::InvalidRequest(format!(
                "pool {pool_id} already exists"
            )));
        }
        map.insert(pool_id, Arc::new(RwLock::new(entry)));
        Ok(pool_id)
    }

    /// Returns a shared reference to the pool entry behind a per-pool lock.
    ///
    /// # Errors
    ///
    /// Returns [`GatewayError::PoolNotFound`] if no pool with the given ID
    /// exists.
    pub async fn get(&self, pool_id: PoolId) -> Result<Arc<RwLock<PoolEntry>>, GatewayError> {
        let map = self.pools.read().await;
        map.get(&pool_id)
            .cloned()
            .ok_or(GatewayError::PoolNotFound(*pool_id.as_uuid()))
    }

    /// Removes a pool from the registry, returning its entry.
    ///
    /// # Errors
    ///
    /// Returns [`GatewayError::PoolNotFound`] if no pool with the given ID
    /// exists.
    pub async fn remove(&self, pool_id: PoolId) -> Result<PoolEntry, GatewayError> {
        let mut map = self.pools.write().await;
        let arc = map
            .remove(&pool_id)
            .ok_or(GatewayError::PoolNotFound(*pool_id.as_uuid()))?;
        // Unwrap the Arc — we just removed it so we hold the only strong ref
        // after the map write lock is released. Use `try_unwrap` to be safe.
        let entry = Arc::try_unwrap(arc)
            .map_err(|_| {
                GatewayError::Internal("pool entry still referenced elsewhere".to_string())
            })?
            .into_inner();
        Ok(entry)
    }

    /// Returns summaries of all pools, optionally filtered by pool type.
    pub async fn list(&self, pool_type_filter: Option<&str>) -> Vec<PoolSummary> {
        let map = self.pools.read().await;
        let mut summaries = Vec::with_capacity(map.len());
        for entry_lock in map.values() {
            let entry = entry_lock.read().await;
            if let Some(filter) = pool_type_filter
                && entry.pool_type != filter
            {
                continue;
            }
            summaries.push(PoolSummary::from(&*entry));
        }
        summaries
    }

    /// Returns the number of pools in the registry.
    pub async fn len(&self) -> usize {
        self.pools.read().await.len()
    }

    /// Returns `true` if the registry contains no pools.
    pub async fn is_empty(&self) -> bool {
        self.pools.read().await.is_empty()
    }
}

impl Default for PoolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
#[allow(clippy::panic)]
mod tests {
    use super::*;
    use hydra_amm::config::{AmmConfig, ConstantProductConfig};
    use hydra_amm::domain::{
        Amount, BasisPoints, Decimals, FeeTier, Token, TokenAddress, TokenPair,
    };
    use hydra_amm::factory::DefaultPoolFactory;

    fn make_pool_entry() -> PoolEntry {
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
        let config = AmmConfig::ConstantProduct(cfg);
        let Ok(pool_box) = DefaultPoolFactory::create(&config) else {
            panic!("pool creation failed");
        };
        PoolEntry::new(PoolId::new(), pool_box, "constant_product".to_string(), 30)
    }

    #[tokio::test]
    async fn insert_and_get() {
        let registry = PoolRegistry::new();
        let entry = make_pool_entry();
        let id = entry.pool_id;

        let result = registry.insert(entry).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap_or_default(), id);

        let fetched = registry.get(id).await;
        assert!(fetched.is_ok());
    }

    #[tokio::test]
    async fn get_nonexistent_returns_error() {
        let registry = PoolRegistry::new();
        let result = registry.get(PoolId::new()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn remove_returns_entry() {
        let registry = PoolRegistry::new();
        let entry = make_pool_entry();
        let id = entry.pool_id;

        let _ = registry.insert(entry).await;
        let removed = registry.remove(id).await;
        assert!(removed.is_ok());

        // Now it should be gone
        let result = registry.get(id).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn remove_nonexistent_returns_error() {
        let registry = PoolRegistry::new();
        let result = registry.remove(PoolId::new()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn list_returns_all() {
        let registry = PoolRegistry::new();
        let _ = registry.insert(make_pool_entry()).await;
        let _ = registry.insert(make_pool_entry()).await;

        let list = registry.list(None).await;
        assert_eq!(list.len(), 2);
    }

    #[tokio::test]
    async fn list_filters_by_type() {
        let registry = PoolRegistry::new();
        let _ = registry.insert(make_pool_entry()).await;

        let matched = registry.list(Some("constant_product")).await;
        assert_eq!(matched.len(), 1);

        let unmatched = registry.list(Some("clmm")).await;
        assert!(unmatched.is_empty());
    }

    #[tokio::test]
    async fn len_and_is_empty() {
        let registry = PoolRegistry::new();
        assert!(registry.is_empty().await);
        assert_eq!(registry.len().await, 0);

        let _ = registry.insert(make_pool_entry()).await;
        assert!(!registry.is_empty().await);
        assert_eq!(registry.len().await, 1);
    }
}
