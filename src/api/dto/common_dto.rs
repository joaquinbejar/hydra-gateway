//! Shared DTO types used across multiple endpoints.

use serde::{Deserialize, Serialize};

/// Token metadata as provided in pool creation requests.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TokenDto {
    /// Hex-encoded token address.
    pub address: String,
    /// Number of decimal places.
    pub decimals: u8,
    /// Human-readable token symbol (e.g. `"ETH"`).
    #[serde(default)]
    pub symbol: String,
}

/// Pagination query parameters for list endpoints.
#[derive(Debug, Clone, Deserialize)]
pub struct PaginationParams {
    /// Page number (1-indexed). Defaults to 1.
    #[serde(default = "default_page")]
    pub page: u32,
    /// Items per page (max 100). Defaults to 20.
    #[serde(default = "default_per_page")]
    pub per_page: u32,
}

/// Pagination metadata included in list responses.
#[derive(Debug, Clone, Serialize)]
pub struct PaginationMeta {
    /// Current page number.
    pub page: u32,
    /// Items per page.
    pub per_page: u32,
    /// Total number of items.
    pub total: u32,
    /// Total number of pages.
    pub total_pages: u32,
}

fn default_page() -> u32 {
    1
}

fn default_per_page() -> u32 {
    20
}

impl PaginationParams {
    /// Clamps `per_page` to the allowed maximum of 100.
    #[must_use]
    pub fn clamped(&self) -> Self {
        Self {
            page: self.page.max(1),
            per_page: self.per_page.clamp(1, 100),
        }
    }
}
