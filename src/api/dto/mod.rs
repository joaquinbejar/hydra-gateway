//! Data Transfer Objects for REST request/response serialization.
//!
//! All numeric amounts are serialized as JSON strings to prevent
//! precision loss on u128 values.

pub mod common_dto;
pub mod liquidity_dto;
pub mod pool_dto;
pub mod swap_dto;

pub use common_dto::*;
pub use liquidity_dto::*;
pub use pool_dto::*;
pub use swap_dto::*;
