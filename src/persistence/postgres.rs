//! PostgreSQL implementation of the persistence layer.

use chrono::{DateTime, Utc};
use sqlx::PgPool;
use uuid::Uuid;

use super::models::{PoolSnapshot, StoredEvent};
use crate::error::GatewayError;

/// PostgreSQL-backed persistence layer using `sqlx::PgPool`.
#[derive(Debug, Clone)]
pub struct PostgresPersistence {
    pool: PgPool,
}

impl PostgresPersistence {
    /// Creates a new persistence layer with the given connection pool.
    #[must_use]
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Appends an event to the event log.
    ///
    /// # Errors
    ///
    /// Returns a [`GatewayError::PersistenceError`] on database failure.
    pub async fn save_event(
        &self,
        pool_id: Uuid,
        event_type: &str,
        payload: &serde_json::Value,
    ) -> Result<i64, GatewayError> {
        let row = sqlx::query_scalar::<_, i64>(
            "INSERT INTO events (pool_id, event_type, payload) VALUES ($1, $2, $3) RETURNING id",
        )
        .bind(pool_id)
        .bind(event_type)
        .bind(payload)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| GatewayError::PersistenceError(e.to_string()))?;

        Ok(row)
    }

    /// Saves a pool state snapshot.
    ///
    /// # Errors
    ///
    /// Returns a [`GatewayError::PersistenceError`] on database failure.
    pub async fn save_snapshot(
        &self,
        pool_id: Uuid,
        pool_type: &str,
        config_json: &serde_json::Value,
        state_json: &serde_json::Value,
        metadata_json: &serde_json::Value,
    ) -> Result<i64, GatewayError> {
        let row = sqlx::query_scalar::<_, i64>(
            "INSERT INTO pool_snapshots (pool_id, pool_type, config_json, state_json, metadata_json) \
             VALUES ($1, $2, $3, $4, $5) RETURNING id",
        )
        .bind(pool_id)
        .bind(pool_type)
        .bind(config_json)
        .bind(state_json)
        .bind(metadata_json)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| GatewayError::PersistenceError(e.to_string()))?;

        Ok(row)
    }

    /// Loads the latest snapshot for each pool using `DISTINCT ON`.
    ///
    /// # Errors
    ///
    /// Returns a [`GatewayError::PersistenceError`] on database failure.
    pub async fn load_latest_snapshots(&self) -> Result<Vec<PoolSnapshot>, GatewayError> {
        let rows = sqlx::query_as::<_, (i64, Uuid, String, serde_json::Value, serde_json::Value, serde_json::Value, DateTime<Utc>)>(
            "SELECT DISTINCT ON (pool_id) id, pool_id, pool_type, config_json, state_json, metadata_json, snapshot_at \
             FROM pool_snapshots ORDER BY pool_id, snapshot_at DESC",
        )
        .fetch_all(&self.pool)
        .await
        .map_err(|e| GatewayError::PersistenceError(e.to_string()))?;

        Ok(rows
            .into_iter()
            .map(
                |(id, pool_id, pool_type, config_json, state_json, metadata_json, snapshot_at)| {
                    PoolSnapshot {
                        id,
                        pool_id,
                        pool_type,
                        config_json,
                        state_json,
                        metadata_json,
                        snapshot_at,
                    }
                },
            )
            .collect())
    }

    /// Loads events after the given timestamp, optionally filtered by pool ID.
    ///
    /// # Errors
    ///
    /// Returns a [`GatewayError::PersistenceError`] on database failure.
    pub async fn load_events_after(
        &self,
        after: DateTime<Utc>,
        pool_id: Option<Uuid>,
    ) -> Result<Vec<StoredEvent>, GatewayError> {
        let rows = if let Some(pid) = pool_id {
            sqlx::query_as::<_, (i64, Uuid, String, serde_json::Value, DateTime<Utc>)>(
                "SELECT id, pool_id, event_type, payload, created_at FROM events \
                 WHERE created_at > $1 AND pool_id = $2 ORDER BY created_at ASC",
            )
            .bind(after)
            .bind(pid)
            .fetch_all(&self.pool)
            .await
        } else {
            sqlx::query_as::<_, (i64, Uuid, String, serde_json::Value, DateTime<Utc>)>(
                "SELECT id, pool_id, event_type, payload, created_at FROM events \
                 WHERE created_at > $1 ORDER BY created_at ASC",
            )
            .bind(after)
            .fetch_all(&self.pool)
            .await
        }
        .map_err(|e| GatewayError::PersistenceError(e.to_string()))?;

        Ok(rows
            .into_iter()
            .map(
                |(id, pool_id, event_type, payload, created_at)| StoredEvent {
                    id,
                    pool_id,
                    event_type,
                    payload,
                    created_at,
                },
            )
            .collect())
    }

    /// Deletes snapshots older than the given number of days.
    ///
    /// # Errors
    ///
    /// Returns a [`GatewayError::PersistenceError`] on database failure.
    pub async fn delete_old_snapshots(&self, before_days: u64) -> Result<u64, GatewayError> {
        let cutoff =
            Utc::now() - chrono::Duration::days(i64::try_from(before_days).unwrap_or(i64::MAX));

        let result = sqlx::query("DELETE FROM pool_snapshots WHERE snapshot_at < $1")
            .bind(cutoff)
            .execute(&self.pool)
            .await
            .map_err(|e| GatewayError::PersistenceError(e.to_string()))?;

        Ok(result.rows_affected())
    }
}
