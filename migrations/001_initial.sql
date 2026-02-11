-- hydra-gateway initial schema
-- Creates events and pool_snapshots tables with indexes.

CREATE TABLE events (
    id          BIGSERIAL PRIMARY KEY,
    pool_id     UUID NOT NULL,
    event_type  VARCHAR(64) NOT NULL,
    payload     JSONB NOT NULL,
    created_at  TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Composite index for time-range queries filtered by pool
CREATE INDEX idx_events_pool_id_created ON events (pool_id, created_at);

-- Index for querying by event type (monitoring, analytics)
CREATE INDEX idx_events_type ON events (event_type);

CREATE TABLE pool_snapshots (
    id            BIGSERIAL PRIMARY KEY,
    pool_id       UUID NOT NULL,
    pool_type     VARCHAR(64) NOT NULL,
    config_json   JSONB NOT NULL,
    state_json    JSONB NOT NULL,
    metadata_json JSONB NOT NULL,
    snapshot_at   TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Find latest snapshot per pool efficiently
CREATE INDEX idx_snapshots_pool_id_at ON pool_snapshots (pool_id, snapshot_at DESC);

-- Clean up old snapshots
CREATE INDEX idx_snapshots_age ON pool_snapshots (snapshot_at DESC);
