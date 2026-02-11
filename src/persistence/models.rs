//! Database models for events and snapshots.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A stored event row from the `events` table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredEvent {
    /// Auto-increment row ID.
    pub id: i64,
    /// Pool that generated the event.
    pub pool_id: Uuid,
    /// Event type discriminator (e.g. `"swap_executed"`).
    pub event_type: String,
    /// JSONB payload with event-specific data.
    pub payload: serde_json::Value,
    /// Server-side creation timestamp.
    pub created_at: DateTime<Utc>,
}

/// A pool snapshot row from the `pool_snapshots` table.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PoolSnapshot {
    /// Auto-increment row ID.
    pub id: i64,
    /// Pool that was snapshotted.
    pub pool_id: Uuid,
    /// Pool type string.
    pub pool_type: String,
    /// Pool configuration as JSONB.
    pub config_json: serde_json::Value,
    /// Full pool state as JSONB.
    pub state_json: serde_json::Value,
    /// Pool metadata as JSONB.
    pub metadata_json: serde_json::Value,
    /// Snapshot timestamp.
    pub snapshot_at: DateTime<Utc>,
}
