# =============================================================================
# Stage 1: Build
# =============================================================================
FROM rust:1.93-alpine3.23 AS builder

RUN apk add --no-cache musl-dev pkgconfig openssl-dev openssl-libs-static curl

WORKDIR /build

# Copy full source and build
COPY . .

RUN cargo build --release
RUN strip target/release/hydra-gateway

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
