-- Roles Postgres para TrenchPass · NO ejecutar como migración runtime.
-- Aplicar manualmente con superusuario tras provisionar la BD.

DO $$
BEGIN
  IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'audit_writer') THEN
    CREATE ROLE audit_writer LOGIN PASSWORD 'CHANGE_IN_ENV';
  END IF;
END
$$;

GRANT CONNECT ON DATABASE audit TO audit_writer;
GRANT USAGE ON SCHEMA public TO audit_writer;
GRANT INSERT ON audit_events TO audit_writer;
GRANT USAGE ON SEQUENCE audit_events_id_seq TO audit_writer;

DO $$
BEGIN
  IF NOT EXISTS (SELECT 1 FROM pg_roles WHERE rolname = 'audit_reader') THEN
    CREATE ROLE audit_reader LOGIN PASSWORD 'CHANGE_IN_ENV';
  END IF;
END
$$;

GRANT CONNECT ON DATABASE audit TO audit_reader;
GRANT USAGE ON SCHEMA public TO audit_reader;
GRANT SELECT ON audit_events TO audit_reader;
