//! Type-safe pool identifier.
//!
//! [`PoolId`] is a newtype wrapper around [`uuid::Uuid`] (v4) providing
//! type safety so that pool identifiers cannot be confused with other UUIDs.

use std::fmt;

use serde::{Deserialize, Serialize};

/// Unique identifier for an AMM pool.
///
/// Wraps a UUID v4. Generated once at pool creation time and immutable
/// thereafter. Used as the dictionary key in [`super::PoolRegistry`],
/// event discriminator, and WebSocket subscription target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PoolId(uuid::Uuid);

impl PoolId {
    /// Creates a new random `PoolId` (UUID v4).
    #[must_use]
    pub fn new() -> Self {
        Self(uuid::Uuid::new_v4())
    }

    /// Creates a `PoolId` from an existing [`uuid::Uuid`].
    #[must_use]
    pub const fn from_uuid(uuid: uuid::Uuid) -> Self {
        Self(uuid)
    }

    /// Returns the inner [`uuid::Uuid`].
    #[must_use]
    pub const fn as_uuid(&self) -> &uuid::Uuid {
        &self.0
    }
}

impl Default for PoolId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for PoolId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl From<uuid::Uuid> for PoolId {
    fn from(uuid: uuid::Uuid) -> Self {
        Self(uuid)
    }
}

impl From<PoolId> for uuid::Uuid {
    fn from(id: PoolId) -> Self {
        id.0
    }
}

#[cfg(test)]
#[allow(clippy::panic)]
mod tests {
    use super::*;

    #[test]
    fn new_generates_unique_ids() {
        let a = PoolId::new();
        let b = PoolId::new();
        assert_ne!(a, b);
    }

    #[test]
    fn display_is_uuid_format() {
        let id = PoolId::new();
        let s = format!("{id}");
        assert_eq!(s.len(), 36); // UUID string length
        assert!(s.contains('-'));
    }

    #[test]
    fn serde_round_trip() {
        let id = PoolId::new();
        let json = serde_json::to_string(&id).ok();
        let Some(json) = json else {
            panic!("serialization failed");
        };
        let deserialized: PoolId = serde_json::from_str(&json).ok().unwrap_or_else(|| {
            panic!("deserialization failed");
        });
        assert_eq!(id, deserialized);
    }

    #[test]
    fn from_uuid_round_trip() {
        let uuid = uuid::Uuid::new_v4();
        let id = PoolId::from_uuid(uuid);
        assert_eq!(*id.as_uuid(), uuid);
    }

    #[test]
    fn hash_works_in_hashmap() {
        use std::collections::HashMap;
        let id = PoolId::new();
        let mut map = HashMap::new();
        map.insert(id, "test");
        assert_eq!(map.get(&id), Some(&"test"));
    }

    #[test]
    fn default_creates_new() {
        let a = PoolId::default();
        let b = PoolId::default();
        assert_ne!(a, b);
    }
}
