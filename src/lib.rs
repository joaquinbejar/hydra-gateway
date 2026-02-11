//! # hydra-gateway
//!
//! REST API and WebSocket gateway for the `hydra-amm` universal AMM engine.
//!
//! This crate provides an HTTP and WebSocket interface for creating, managing,
//! and interacting with any supported AMM pool type. All AMM mathematics are
//! delegated to `hydra-amm` — this service is a coordination layer.
//!
//! ## Architecture
//!
//! ```text
//! Clients (HTTP, WebSocket)
//!     │
//!     ├── REST Handlers (api/)
//!     ├── WS Handler (ws/)
//!     │
//!     ├── PoolService (service/)
//!     ├── EventBus (domain/)
//!     │
//!     ├── PoolRegistry (domain/)
//!     ├── hydra-amm (PoolBox)
//!     │
//!     └── PostgreSQL Persistence
//! ```

pub mod api;
pub mod app_state;
pub mod config;
pub mod domain;
pub mod error;
pub mod persistence;
pub mod service;
pub mod ws;
