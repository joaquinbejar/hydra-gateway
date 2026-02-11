//! Domain layer: core types, pool registry, and event system.
//!
//! This module contains the server-side domain model including pool
//! identity, pool entries with metadata, the event bus for broadcasting
//! state changes, and the pool registry for concurrent pool storage.

pub mod event_bus;
pub mod pool_entry;
pub mod pool_event;
pub mod pool_id;
pub mod pool_registry;

pub use event_bus::EventBus;
pub use pool_entry::PoolEntry;
pub use pool_event::PoolEvent;
pub use pool_id::PoolId;
pub use pool_registry::PoolRegistry;
