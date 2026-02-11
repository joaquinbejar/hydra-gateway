//! Persistence layer: PostgreSQL event log and pool snapshots.
//!
//! Provides the `PersistenceLayer` trait for durable storage of pool
//! events and periodic state snapshots. The concrete implementation
//! uses `sqlx::PgPool` for async PostgreSQL access.

pub mod models;
pub mod postgres;
