-- Esquema audit log append-only para MCP Gateway Rust.
-- Toda escritura es INSERT; UPDATE y DELETE están prohibidos por policy.

CREATE TABLE IF NOT EXISTS audit_events (
    id          BIGSERIAL PRIMARY KEY,
    ts          TIMESTAMPTZ NOT NULL DEFAULT now(),
    consumer_id TEXT        NOT NULL,           -- CN del certificado mTLS cliente
    action      TEXT        NOT NULL,           -- read_secret | rotate_secret | health
    namespace   TEXT        NOT NULL,           -- vault namespace / path prefix
    secret_path TEXT        NOT NULL,
    outcome     TEXT        NOT NULL CHECK (outcome IN ('ok','error','denied')),
    latency_ms  INT,
    detail      JSONB
);

-- Índices mínimos para queries frecuentes del dashboard SigNoz/Controlink.
CREATE INDEX IF NOT EXISTS idx_audit_events_ts ON audit_events (ts DESC);
CREATE INDEX IF NOT EXISTS idx_audit_events_consumer ON audit_events (consumer_id, ts DESC);

-- Rol de solo escritura para el gateway (sin SELECT, sin DELETE).
DO $$
BEGIN
  IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'audit_writer') THEN
    CREATE ROLE audit_writer LOGIN PASSWORD 'CHANGE_IN_ENV';
  END IF;
END
$$;

GRANT CONNECT ON DATABASE audit ON SCHEMA public TO audit_writer;
GRANT INSERT ON audit_events TO audit_writer;
GRANT USAGE ON SEQUENCE audit_events_id_seq TO audit_writer;

-- Rol de solo lectura para Controlink Dashboard.
DO $$
BEGIN
  IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'audit_reader') THEN
    CREATE ROLE audit_reader LOGIN PASSWORD 'CHANGE_IN_ENV';
  END IF;
END
$$;

GRANT CONNECT ON DATABASE audit ON SCHEMA public TO audit_reader;
GRANT SELECT ON audit_events TO audit_reader;
