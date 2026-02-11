//! Per-connection subscription manager.
//!
//! Tracks which pool IDs a WebSocket client is subscribed to and
//! provides server-side event filtering.

use std::collections::HashSet;

use crate::domain::PoolId;

/// Manages the set of pool subscriptions for a single WebSocket connection.
#[derive(Debug, Default)]
pub struct SubscriptionManager {
    /// Subscribed pool IDs. If `subscribe_all` is true, this set is ignored.
    pool_ids: HashSet<PoolId>,
    /// Whether the client subscribes to all pools (wildcard `"*"`).
    subscribe_all: bool,
}

impl SubscriptionManager {
    /// Creates a new empty subscription manager.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds pool IDs to the subscription set. `"*"` enables the wildcard.
    pub fn subscribe(&mut self, ids: &[PoolId], wildcard: bool) {
        if wildcard {
            self.subscribe_all = true;
        }
        for id in ids {
            self.pool_ids.insert(*id);
        }
    }

    /// Removes pool IDs from the subscription set.
    pub fn unsubscribe(&mut self, ids: &[PoolId]) {
        for id in ids {
            self.pool_ids.remove(id);
        }
    }

    /// Returns `true` if the given pool ID matches the subscription filter.
    #[must_use]
    pub fn matches(&self, pool_id: PoolId) -> bool {
        self.subscribe_all || self.pool_ids.contains(&pool_id)
    }

    /// Returns the number of explicitly subscribed pool IDs.
    #[must_use]
    pub fn count(&self) -> usize {
        self.pool_ids.len()
    }

    /// Returns `true` if the wildcard subscription is active.
    #[must_use]
    pub fn is_subscribed_all(&self) -> bool {
        self.subscribe_all
    }
}

#[cfg(test)]
#[allow(clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn empty_matches_nothing() {
        let mgr = SubscriptionManager::new();
        assert!(!mgr.matches(PoolId::new()));
    }

    #[test]
    fn subscribe_specific_pool() {
        let mut mgr = SubscriptionManager::new();
        let id = PoolId::new();
        mgr.subscribe(&[id], false);
        assert!(mgr.matches(id));
        assert!(!mgr.matches(PoolId::new()));
    }

    #[test]
    fn wildcard_matches_everything() {
        let mut mgr = SubscriptionManager::new();
        mgr.subscribe(&[], true);
        assert!(mgr.matches(PoolId::new()));
        assert!(mgr.matches(PoolId::new()));
    }

    #[test]
    fn unsubscribe_removes_pool() {
        let mut mgr = SubscriptionManager::new();
        let id = PoolId::new();
        mgr.subscribe(&[id], false);
        assert!(mgr.matches(id));
        mgr.unsubscribe(&[id]);
        assert!(!mgr.matches(id));
    }

    #[test]
    fn count_tracks_explicit() {
        let mut mgr = SubscriptionManager::new();
        assert_eq!(mgr.count(), 0);
        mgr.subscribe(&[PoolId::new(), PoolId::new()], false);
        assert_eq!(mgr.count(), 2);
    }
}
