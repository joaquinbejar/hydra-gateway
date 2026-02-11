# =============================================================================
# Stage 1: Build
# =============================================================================
FROM rust:1.93-alpine3.23 AS builder

RUN apk add --no-cache musl-dev pkgconfig openssl-dev openssl-libs-static

WORKDIR /build

# Cache dependencies: copy manifests first
COPY Cargo.toml rust-toolchain.toml ./

# Create a dummy main.rs to pre-build dependencies
RUN mkdir src && \
    echo 'fn main() {}' > src/main.rs && \
    echo '//! stub' > src/lib.rs && \
    cargo build --release 2>/dev/null || true && \
    rm -rf src

# Copy full source and build
COPY src/ src/
COPY migrations/ migrations/

RUN cargo build --release --locked 2>/dev/null || cargo build --release && \
    strip target/release/hydra-gateway

# =============================================================================
# Stage 2: Runtime
# =============================================================================
FROM alpine:3.23

RUN apk add --no-cache ca-certificates libgcc

RUN addgroup -S hydra && adduser -S hydra -G hydra

COPY --from=builder /build/target/release/hydra-gateway /usr/local/bin/hydra-gateway
COPY migrations/ /app/migrations/

WORKDIR /app

USER hydra

EXPOSE 3000

ENV LISTEN_ADDR=0.0.0.0:3000 \
    RUST_LOG=info

ENTRYPOINT ["hydra-gateway"]
