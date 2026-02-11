//! Service layer: business logic orchestration.
//!
//! [`PoolService`] coordinates pool operations, delegates computation
//! to hydra-amm, and emits events through the [`super::domain::EventBus`].

pub mod pool_service;

pub use pool_service::PoolService;
