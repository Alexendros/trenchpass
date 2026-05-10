//! Persistencia append-only del audit log.
//!
//! El rol `audit_writer` solo tiene `INSERT` + `USAGE` sobre la secuencia.
//! Cualquier intento de UPDATE/DELETE retornará error de Postgres y se reflejará en SigNoz.

use serde::Serialize;
use sqlx::postgres::{PgPool, PgPoolOptions};
use tracing::instrument;

use crate::config::DatabaseConfig;
use crate::error::Result;

#[derive(Debug, Clone, Copy, Serialize)]
pub enum AuditOutcome {
    Ok,
    Error,
    Denied,
}

impl AuditOutcome {
    fn as_str(self) -> &'static str {
        match self {
            Self::Ok => "ok",
            Self::Error => "error",
            Self::Denied => "denied",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AuditEvent<'a> {
    pub consumer_id: &'a str,
    pub action: &'a str,
    pub namespace: &'a str,
    pub secret_path: &'a str,
    pub outcome: AuditOutcome,
    pub latency_ms: Option<i32>,
    pub detail: Option<serde_json::Value>,
}

#[derive(Clone)]
pub struct AuditStore {
    pool: PgPool,
}

impl AuditStore {
    pub async fn connect(cfg: &DatabaseConfig) -> Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(cfg.max_connections)
            .acquire_timeout(std::time::Duration::from_secs(5))
            .connect(&cfg.url)
            .await?;
        Ok(Self { pool })
    }

    #[instrument(
        skip(self, event),
        fields(
            consumer = %event.consumer_id,
            action = %event.action,
            namespace = %event.namespace,
            outcome = ?event.outcome,
        )
    )]
    /// `audit_writer` solo tiene `INSERT` → no usamos `RETURNING` (requiere SELECT).
    /// El id queda asignado por la secuencia y el dashboard Controlink lo lee con el rol `audit_reader`.
    pub async fn record(&self, event: AuditEvent<'_>) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO audit_events
                (consumer_id, action, namespace, secret_path, outcome, latency_ms, detail)
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            "#,
        )
        .bind(event.consumer_id)
        .bind(event.action)
        .bind(event.namespace)
        .bind(event.secret_path)
        .bind(event.outcome.as_str())
        .bind(event.latency_ms)
        .bind(event.detail.clone())
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
