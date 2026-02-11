//! Gateway configuration loaded from environment variables.
//!
//! Follows 12-factor style: all settings come from environment variables
//! (or a `.env` file via `dotenvy`). See `04-PERSISTENCE.md` for the
//! full list of configuration keys.

use std::net::SocketAddr;

/// Top-level gateway configuration.
///
/// Loaded once at startup via [`GatewayConfig::from_env`].
#[derive(Debug, Clone)]
pub struct GatewayConfig {
    /// Socket address to bind the HTTP server to (e.g. `0.0.0.0:3000`).
    pub listen_addr: SocketAddr,

    /// PostgreSQL connection string.
    pub database_url: String,

    /// Maximum number of database connections in the pool.
    pub database_max_connections: u32,

    /// Minimum idle connections in the pool.
    pub database_min_connections: u32,

    /// Timeout in seconds for acquiring a database connection.
    pub database_connect_timeout_secs: u64,

    /// Master switch for the persistence layer.
    pub persistence_enabled: bool,

    /// Seconds between automatic pool snapshots.
    pub snapshot_interval_secs: u64,

    /// Whether to append events to the event log.
    pub event_log_enabled: bool,

    /// Delete snapshots older than this many days (0 = never).
    pub cleanup_after_days: u64,

    /// Capacity of the EventBus broadcast channel.
    pub event_bus_capacity: usize,
}

impl GatewayConfig {
    /// Loads configuration from environment variables.
    ///
    /// Falls back to sensible defaults when a variable is not set.
    /// Calls `dotenvy::dotenv().ok()` to optionally load a `.env` file.
    ///
    /// # Errors
    ///
    /// Returns an error if `LISTEN_ADDR` is set but cannot be parsed as
    /// a [`SocketAddr`].
    pub fn from_env() -> Result<Self, Box<dyn std::error::Error>> {
        dotenvy::dotenv().ok();

        let listen_addr: SocketAddr = std::env::var("LISTEN_ADDR")
            .unwrap_or_else(|_| "0.0.0.0:3000".to_string())
            .parse()?;

        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://hydra:hydra@localhost:5432/hydra_gateway".to_string());

        let database_max_connections = parse_env("DATABASE_MAX_CONNECTIONS", 10);
        let database_min_connections = parse_env("DATABASE_MIN_CONNECTIONS", 2);
        let database_connect_timeout_secs = parse_env("DATABASE_CONNECT_TIMEOUT_SECS", 5);

        let persistence_enabled = parse_env_bool("PERSISTENCE_ENABLED", true);
        let snapshot_interval_secs = parse_env("PERSISTENCE_SNAPSHOT_INTERVAL_SECS", 60);
        let event_log_enabled = parse_env_bool("PERSISTENCE_EVENT_LOG_ENABLED", true);
        let cleanup_after_days = parse_env("PERSISTENCE_CLEANUP_AFTER_DAYS", 30);

        let event_bus_capacity = parse_env("EVENT_BUS_CAPACITY", 10_000);

        Ok(Self {
            listen_addr,
            database_url,
            database_max_connections,
            database_min_connections,
            database_connect_timeout_secs,
            persistence_enabled,
            snapshot_interval_secs,
            event_log_enabled,
            cleanup_after_days,
            event_bus_capacity,
        })
    }
}

/// Parses an environment variable as `T`, returning `default` on missing
/// or invalid values.
fn parse_env<T: std::str::FromStr>(key: &str, default: T) -> T {
    std::env::var(key)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

/// Parses an environment variable as a boolean. Accepts `"true"`, `"1"`,
/// `"false"`, `"0"` (case-insensitive). Returns `default` otherwise.
fn parse_env_bool(key: &str, default: bool) -> bool {
    match std::env::var(key).ok().as_deref() {
        Some("true") | Some("TRUE") | Some("1") => true,
        Some("false") | Some("FALSE") | Some("0") => false,
        _ => default,
    }
}
