//! Gateway error types with HTTP status code mapping.
//!
//! [`GatewayError`] is the central error type for the gateway. Each variant
//! maps to a specific HTTP status code and structured JSON error response.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde::Serialize;

/// Structured JSON error response body.
///
/// All error responses follow this shape:
/// ```json
/// {
///   "error": {
///     "code": 1001,
///     "message": "Invalid price: must be positive",
///     "details": null
///   }
/// }
/// ```
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    /// Structured error payload.
    pub error: ErrorBody,
}

/// Inner error body with numeric code and human-readable message.
#[derive(Debug, Serialize)]
pub struct ErrorBody {
    /// Numeric error code (see code ranges in spec).
    pub code: u32,
    /// Human-readable error message.
    pub message: String,
    /// Optional additional details.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<String>,
}

/// Server-side error enum with HTTP status code mapping.
///
/// # Error Code Ranges
///
/// | Range     | Category        | HTTP Status                |
/// |-----------|-----------------|----------------------------|
/// | 1000–1999 | Validation      | 400 Bad Request            |
/// | 2000–2999 | State/Not Found | 404 Not Found / 409 Conflict |
/// | 3000–3999 | Server          | 500 Internal Server Error  |
/// | 4000–4999 | Pool-Specific   | 422 Unprocessable Entity   |
#[derive(Debug, thiserror::Error)]
pub enum GatewayError {
    /// Pool with the given ID was not found.
    #[error("pool not found: {0}")]
    PoolNotFound(uuid::Uuid),

    /// Request validation failed.
    #[error("invalid request: {0}")]
    InvalidRequest(String),

    /// Pool does not have enough liquidity for the operation.
    #[error("insufficient liquidity in pool")]
    InsufficientLiquidity,

    /// Insufficient balance for the requested token.
    #[error("insufficient balance: {0}")]
    InsufficientBalance(String),

    /// Liquidity position not found.
    #[error("position not found in pool {0}")]
    PositionNotFound(uuid::Uuid),

    /// Error propagated from the hydra-amm computation engine.
    #[error("amm error: {0}")]
    AmmError(#[from] hydra_amm::error::AmmError),

    /// Persistence layer failure.
    #[error("persistence error: {0}")]
    PersistenceError(String),

    /// Client exceeded rate limit.
    #[error("rate limit exceeded; retry after {retry_after_ms} ms")]
    RateLimited {
        /// Milliseconds until the client may retry.
        retry_after_ms: u64,
    },

    /// Unsupported or invalid pool type string.
    #[error("invalid pool type: {0}")]
    InvalidPoolType(String),

    /// Internal server error.
    #[error("internal error: {0}")]
    Internal(String),
}

impl GatewayError {
    /// Returns the numeric error code for this variant.
    #[must_use]
    pub const fn error_code(&self) -> u32 {
        match self {
            Self::InvalidRequest(_) => 1001,
            Self::InvalidPoolType(_) => 1002,
            Self::PoolNotFound(_) => 2001,
            Self::PositionNotFound(_) => 2002,
            Self::InsufficientLiquidity => 4001,
            Self::InsufficientBalance(_) => 4002,
            Self::AmmError(_) => 1003,
            Self::PersistenceError(_) => 3001,
            Self::RateLimited { .. } => 429,
            Self::Internal(_) => 3000,
        }
    }

    /// Returns the HTTP status code for this variant.
    #[must_use]
    pub const fn status_code(&self) -> StatusCode {
        match self {
            Self::InvalidRequest(_) | Self::InvalidPoolType(_) | Self::AmmError(_) => {
                StatusCode::BAD_REQUEST
            }
            Self::PoolNotFound(_) | Self::PositionNotFound(_) => StatusCode::NOT_FOUND,
            Self::InsufficientLiquidity | Self::InsufficientBalance(_) => {
                StatusCode::UNPROCESSABLE_ENTITY
            }
            Self::PersistenceError(_) | Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
            Self::RateLimited { .. } => StatusCode::TOO_MANY_REQUESTS,
        }
    }
}

impl IntoResponse for GatewayError {
    fn into_response(self) -> Response {
        let status = self.status_code();
        let body = ErrorResponse {
            error: ErrorBody {
                code: self.error_code(),
                message: self.to_string(),
                details: None,
            },
        };
        let mut response = axum::Json(body).into_response();
        *response.status_mut() = status;
        response
    }
}
