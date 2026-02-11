# Hydra Gateway

[![Crates.io](https://img.shields.io/crates/v/hydra-gateway.svg)](https://crates.io/crates/hydra-gateway)
[![Documentation](https://docs.rs/hydra-gateway/badge.svg)](https://docs.rs/hydra-gateway)
[![Latest Release](https://img.shields.io/github/v/release/joaquinbejar/hydra-gateway)](https://github.com/joaquinbejar/hydra-gateway/releases)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Build Status](https://github.com/joaquinbejar/hydra-gateway/actions/workflows/ci.yml/badge.svg)](https://github.com/joaquinbejar/hydra-gateway/actions)
[![codecov](https://codecov.io/gh/joaquinbejar/hydra-gateway/branch/main/graph/badge.svg)](https://codecov.io/gh/joaquinbejar/hydra-gateway)
[![Docker](https://img.shields.io/badge/docker-ghcr.io-blue?logo=docker)](https://ghcr.io/joaquinbejar/hydra-gateway)

**REST API and WebSocket gateway** for the [hydra-amm](https://github.com/joaquinbejar/hydra-amm) universal AMM engine.

Hydra Gateway exposes every AMM pool type supported by `hydra-amm` through a JSON REST API and a real-time WebSocket feed. All AMM mathematics are delegated to `hydra-amm` — this service is a thin coordination and persistence layer.

---

## Supported Pool Types

| Pool Type | Description | REST Endpoint |
|-----------|-------------|---------------|
| **Constant Product** | Uniswap V2 style (x · y = k) | `POST /api/v1/pools` |
| **CLMM** | Concentrated Liquidity (Uniswap V3 style) | `POST /api/v1/pools` |
| **Hybrid / StableSwap** | Curve-style with amplification | `POST /api/v1/pools` |
| **Weighted** | Balancer-style multi-token pools | `POST /api/v1/pools` |
| **Dynamic / PMM** | DODO-style oracle-driven pricing | `POST /api/v1/pools` |
| **Order Book** | Phoenix-style CLOB + AMM hybrid | `POST /api/v1/pools` |

---

## Architecture

```
┌─────────────────────────────────────────────────┐
│  Clients (HTTP, WebSocket)                      │
├─────────────────────────────────────────────────┤
│  Layer 5: REST Handlers (api/)                  │
│  Axum routes · DTOs · OpenAPI (Swagger UI)      │
├─────────────────────────────────────────────────┤
│  Layer 4: WebSocket Handler (ws/)               │
│  Subscriptions · Real-time event streaming      │
├─────────────────────────────────────────────────┤
│  Layer 3: PoolService (service/)                │
│  Orchestration · Event emission                 │
├─────────────────────────────────────────────────┤
│  Layer 2: Domain (domain/)                      │
│  PoolRegistry · EventBus · PoolId · PoolEntry   │
├─────────────────────────────────────────────────┤
│  Layer 1: hydra-amm Engine                      │
│  PoolBox enum dispatch · SwapPool · LiquidityPool│
├─────────────────────────────────────────────────┤
│  Layer 0: Persistence (persistence/)            │
│  PostgreSQL · Events log · Pool snapshots       │
└─────────────────────────────────────────────────┘
```

**Key design decisions:**

- **Per-pool `RwLock`**: Fine-grained concurrency — no global mutex.
- **Enum dispatch**: `PoolBox` from `hydra-amm` — zero vtable overhead.
- **String-encoded amounts**: All `u128` values serialized as JSON strings to prevent precision loss.
- **EventBus**: `tokio::broadcast` channel (configurable capacity) for real-time event streaming.
- **OpenAPI documentation**: Full Swagger UI at `/swagger-ui`.

---

## API Endpoints

### System

| Method | Path | Description |
|--------|------|-------------|
| `GET` | `/health` | Health check |
| `GET` | `/config/pool-types` | List supported pool types |

### Pools

| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/api/v1/pools` | Create a new pool |
| `GET` | `/api/v1/pools` | List pools (paginated) |
| `GET` | `/api/v1/pools/{id}` | Get pool details |
| `DELETE` | `/api/v1/pools/{id}` | Delete a pool |

### Swaps

| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/api/v1/pools/{id}/swap` | Execute a swap |
| `POST` | `/api/v1/pools/{id}/quote` | Get swap quote (read-only) |

### Liquidity

| Method | Path | Description |
|--------|------|-------------|
| `POST` | `/api/v1/pools/{id}/liquidity/add` | Add liquidity |
| `POST` | `/api/v1/pools/{id}/liquidity/remove` | Remove liquidity |

### WebSocket

| Path | Description |
|------|-------------|
| `/ws` | Real-time event streaming (subscribe to pool events) |

### Documentation

| Path | Description |
|------|-------------|
| `/swagger-ui` | Interactive Swagger UI |
| `/api-docs/openapi.json` | OpenAPI 3.0 specification |

---

## Quick Start

### Using Docker Compose

The fastest way to get started:

```bash
cd Docker
docker compose up -d
```

This starts PostgreSQL and the gateway. The API is available at `http://localhost:3000`.

### From Source

```bash
# 1. Clone the repository
git clone https://github.com/joaquinbejar/hydra-gateway.git
cd hydra-gateway

# 2. Start PostgreSQL (requires Docker)
cd Docker && docker compose up -d postgres && cd ..

# 3. Copy environment file
cp .env.example .env

# 4. Run the gateway
cargo run
```

### Create a Pool

```bash
curl -X POST http://localhost:3000/api/v1/pools \
  -H "Content-Type: application/json" \
  -d '{
    "pool_type": "constant_product",
    "name": "USDC/WETH",
    "config": {
      "token_a": { "address": "usdc", "decimals": 6 },
      "token_b": { "address": "weth", "decimals": 18 },
      "fee_bps": 30,
      "reserve_a": "1000000",
      "reserve_b": "1000000"
    }
  }'
```

### Execute a Swap

```bash
curl -X POST http://localhost:3000/api/v1/pools/{pool_id}/swap \
  -H "Content-Type: application/json" \
  -d '{
    "token_in": "usdc",
    "token_out": "weth",
    "amount_in": "10000"
  }'
```

---

## Configuration

All settings are loaded from environment variables (or `.env` file). See [`.env.example`](.env.example) for the full list.

| Variable | Default | Description |
|----------|---------|-------------|
| `LISTEN_ADDR` | `0.0.0.0:3000` | Server bind address |
| `DATABASE_URL` | `postgres://hydra:hydra@localhost:5432/hydra_gateway` | PostgreSQL connection string |
| `DATABASE_MAX_CONNECTIONS` | `10` | Max DB pool connections |
| `DATABASE_MIN_CONNECTIONS` | `2` | Min idle DB connections |
| `DATABASE_CONNECT_TIMEOUT_SECS` | `5` | DB connection timeout (seconds) |
| `PERSISTENCE_ENABLED` | `true` | Enable/disable persistence layer |
| `PERSISTENCE_SNAPSHOT_INTERVAL_SECS` | `60` | Pool snapshot interval (seconds) |
| `PERSISTENCE_EVENT_LOG_ENABLED` | `true` | Enable event logging |
| `PERSISTENCE_CLEANUP_AFTER_DAYS` | `30` | Auto-delete snapshots older than N days |
| `EVENT_BUS_CAPACITY` | `10000` | EventBus broadcast channel capacity |
| `RUST_LOG` | `info` | Log level (tracing format) |

---

## Module Structure

```
hydra_gateway/
├── api/
│   ├── dto/           — Request/response DTOs (all amounts as strings)
│   ├── handlers/      — REST endpoint handlers (system, pool, swap, liquidity)
│   └── mod.rs         — Router composition + OpenAPI (ApiDoc)
├── app_state.rs       — Shared application state (PoolService + EventBus)
├── config.rs          — Environment-based configuration
├── domain/
│   ├── pool_id.rs     — Type-safe UUID v4 pool identifier
│   ├── pool_entry.rs  — Pool metadata wrapper around PoolBox
│   ├── pool_event.rs  — Domain event enum
│   ├── event_bus.rs   — tokio::broadcast event bus
│   └── pool_registry.rs — HashMap<PoolId, RwLock<PoolEntry>>
├── error.rs           — GatewayError → HTTP status code mapping
├── persistence/       — PostgreSQL persistence (events + snapshots)
├── service/
│   └── pool_service.rs — Orchestration layer
└── ws/                — WebSocket handler + subscription manager
```

---

## Docker

### Build the Image

```bash
docker build -f Docker/Dockerfile -t hydra-gateway .
```

### Run with Docker Compose

```bash
cd Docker
docker compose up -d
```

Services:
- **postgres**: PostgreSQL 17 on port `5432`
- **hydra-gateway**: API on port `3000`

---

## Safety and Correctness

- **`unsafe` code is denied** — enforced at the compiler level
- **No `.unwrap()` / `.expect()` / `panic!`** — denied via Clippy lints
- **Per-pool `RwLock`** — no global mutex, no deadlocks
- **Overflow checks** enabled in both debug and release profiles
- **Strict Clippy** with `-D warnings`

---

## Development

### Prerequisites

- Rust 1.93+ (edition 2024, see `rust-toolchain.toml`)
- Docker and Docker Compose (for PostgreSQL)
- Make

### Common Commands

```bash
# Build
make build                   # Debug build
make release                 # Release build

# Test
make test                    # Run all tests
make test-lib                # Library tests only
make test-doc                # Documentation tests

# Quality
make fmt                     # Format code
make lint                    # Run clippy (strict)
make lint-fix                # Auto-fix lint issues
make check                   # fmt-check + lint + test
make pre-push                # Full pre-push validation

# Documentation
make doc                     # Generate docs
make doc-open                # Generate and open in browser

# Docker
make docker-build            # Build Docker image
make docker-up               # Start all services
make docker-down             # Stop all services

# Coverage
make coverage                # Generate XML coverage report
make coverage-html           # Generate HTML coverage report

# Packaging & Publishing
make package                 # List package contents
make publish                 # Dry-run publish to crates.io
make publish-execute         # Publish to crates.io (for real)
```

### Pre-Push Checklist

Always run before pushing:

```bash
make pre-push
```

This executes: `cargo fix` → `cargo fmt` → `cargo clippy` → `cargo test` → `cargo doc`

---

## Release

Releases are triggered by pushing a semver git tag:

```bash
# 1. Update version in Cargo.toml
# 2. Create and push a tag
make tag
git push origin --tags
```

The [release workflow](.github/workflows/release.yml) will automatically:

1. **Validate** — format, clippy, tests, docs
2. **Publish crate** to [crates.io](https://crates.io/crates/hydra-gateway)
3. **Build & push Docker image** to GitHub Container Registry (`ghcr.io`)
4. **Create GitHub Release** with auto-generated changelog

Docker images are available at:

```
ghcr.io/joaquinbejar/hydra-gateway:<version>
ghcr.io/joaquinbejar/hydra-gateway:latest
```

---

## Contributing

Contributions are welcome! Please follow these guidelines:

1. **Fork** the repository and create a feature branch
2. **Read** the documentation in `.internalDoc/`
3. **Write tests** for all new public functions
4. **Run** `make pre-push` before submitting
5. **Create** a PR with a clear description

### Code Standards

- All comments and documentation in **English**
- `///` doc comments on every public item
- `#[must_use]` on all pure functions
- No panics in production code
- Newtypes for all domain concepts

---

## License

This project is licensed under the MIT License — see the [LICENSE](LICENSE) file for details.

---

## Contact

- **Author**: Joaquin Bejar Garcia
- **Email**: jb@taunais.com
- **Telegram**: [@joaquin_bejar](https://t.me/joaquin_bejar)
- **Repository**: [github.com/joaquinbejar/hydra-gateway](https://github.com/joaquinbejar/hydra-gateway)
