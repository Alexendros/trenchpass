-- TrenchPass · audit log append-only.
-- Aplicada por `sqlx::migrate!()` durante `AuditStore::connect`.
-- Los roles `audit_writer` / `audit_reader` viven en `sql/roles.sql` (infra-time, no runtime).

CREATE TABLE IF NOT EXISTS audit_events (
    id          BIGSERIAL PRIMARY KEY,
    ts          TIMESTAMPTZ NOT NULL DEFAULT now(),
    consumer_id TEXT        NOT NULL,
    action      TEXT        NOT NULL,
    namespace   TEXT        NOT NULL,
    secret_path TEXT        NOT NULL,
    outcome     TEXT        NOT NULL CHECK (outcome IN ('ok','error','denied')),
    latency_ms  INT,
    detail      JSONB
);

CREATE INDEX IF NOT EXISTS idx_audit_events_ts ON audit_events (ts DESC);
CREATE INDEX IF NOT EXISTS idx_audit_events_consumer ON audit_events (consumer_id, ts DESC);
CREATE INDEX IF NOT EXISTS idx_audit_events_namespace_ts ON audit_events (namespace, ts DESC);
