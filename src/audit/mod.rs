//! Audit log append-only en Postgres (`audit_events`).
//!
//! Schema definido en `controlink/infra/postgres-audit/init.sql`.
//! El gateway sólo posee privilegio `INSERT`.

pub mod store;

pub use store::{AuditEvent, AuditOutcome, AuditStore};
