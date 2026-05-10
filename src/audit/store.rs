//! Persistencia append-only del audit log.
//!
//! El rol `audit_writer` solo tiene `INSERT` + `USAGE` sobre la secuencia.
//! Cualquier intento de UPDATE/DELETE retornará error de Postgres y se reflejará en SigNoz.
//!
//! Dos modos de escritura:
//! - [`AuditStore::record`]: síncrono, propaga error. Para flujos donde el cliente
//!   debe saber que la escritura falló (rotación de secretos, eventos críticos).
//! - [`AuditStore::record_best_effort`]: tracea el error y devuelve `()`. Para el
//!   hot path de tools, donde un fallo de audit no debe tumbar la respuesta al MCP.

use std::time::Duration;

use serde::Serialize;
use sqlx::postgres::{PgPool, PgPoolOptions};
use tracing::{instrument, warn};

use crate::config::DatabaseConfig;
use crate::error::Result;

/// Timeout duro de la query INSERT. Si Postgres no responde en este tiempo
/// se considera caída del audit log y se devuelve error (síncrono) o se
/// loguea (best-effort). Mantener bajo: el audit no debe ser cuello de botella.
const INSERT_TIMEOUT: Duration = Duration::from_millis(750);

/// Migraciones embebidas. Las aplica [`AuditStore::connect`] al arrancar.
static MIGRATOR: sqlx::migrate::Migrator = sqlx::migrate!("./migrations");

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AuditOutcome {
    Ok,
    Error,
    Denied,
}

impl AuditOutcome {
    pub const fn as_str(self) -> &'static str {
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
            .acquire_timeout(Duration::from_secs(5))
            .connect(&cfg.url)
            .await?;
        MIGRATOR
            .run(&pool)
            .await
            .map_err(|e| crate::error::Error::Audit(sqlx::Error::Migrate(Box::new(e))))?;
        Ok(Self { pool })
    }

    /// Construye un store sobre un pool ya existente. Útil para tests con
    /// `#[sqlx::test]` que inyectan su propio pool efímero.
    #[doc(hidden)]
    pub fn from_pool(pool: PgPool) -> Self {
        Self { pool }
    }

    /// `audit_writer` solo tiene `INSERT` → no usamos `RETURNING` (requiere SELECT).
    /// El id queda asignado por la secuencia y el dashboard Controlink lo lee con el rol `audit_reader`.
    #[instrument(
        skip(self, event),
        fields(
            consumer = %event.consumer_id,
            action = %event.action,
            namespace = %event.namespace,
            outcome = %event.outcome.as_str(),
        )
    )]
    pub async fn record(&self, event: AuditEvent<'_>) -> Result<()> {
        let fut = sqlx::query(
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
        .execute(&self.pool);

        match tokio::time::timeout(INSERT_TIMEOUT, fut).await {
            Ok(Ok(_)) => Ok(()),
            Ok(Err(e)) => Err(crate::error::Error::Audit(e)),
            Err(_) => Err(crate::error::Error::Audit(sqlx::Error::PoolTimedOut)),
        }
    }

    /// Best-effort: no bloquea al caller con errores. Si la escritura falla,
    /// se emite `WARN` con el evento serializado para reproceso offline desde logs.
    pub async fn record_best_effort(&self, event: AuditEvent<'_>) {
        if let Err(e) = self.record(event.clone()).await {
            warn!(
                target: "trenchpass.audit",
                error = %e,
                event = ?serde_json::to_value(&event).ok(),
                "audit write failed · event preserved in logs",
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outcome_str_roundtrip() {
        assert_eq!(AuditOutcome::Ok.as_str(), "ok");
        assert_eq!(AuditOutcome::Error.as_str(), "error");
        assert_eq!(AuditOutcome::Denied.as_str(), "denied");
    }

    #[test]
    fn event_serializes_outcome_lowercase() {
        let ev = AuditEvent {
            consumer_id: "cn=test",
            action: "read_secret",
            namespace: "notion",
            secret_path: "kv/notion/api_key",
            outcome: AuditOutcome::Denied,
            latency_ms: Some(12),
            detail: Some(serde_json::json!({"reason": "scope"})),
        };
        let v = serde_json::to_value(&ev).expect("serialize");
        assert_eq!(v["outcome"], "denied");
        assert_eq!(v["latency_ms"], 12);
    }

    #[test]
    fn insert_timeout_within_bounds() {
        // Sanity: el timeout no es tan agresivo como para quemar SLO ni tan
        // laxo como para volverse el cuello de botella del request.
        assert!(INSERT_TIMEOUT >= Duration::from_millis(250));
        assert!(INSERT_TIMEOUT <= Duration::from_secs(2));
    }
}

#[cfg(all(test, feature = "it-postgres"))]
mod it {
    //! Integration tests · requieren Postgres efímero (`sqlx::test` lo provisiona).
    //! Compilan solo con `cargo test --features it-postgres`.
    use super::*;

    #[sqlx::test(migrations = "./migrations")]
    async fn record_inserts_event(pool: PgPool) {
        let store = AuditStore::from_pool(pool.clone());
        store
            .record(AuditEvent {
                consumer_id: "cn=alexendros-laptop",
                action: "tool_call",
                namespace: "notion",
                secret_path: "kv/notion/api_key",
                outcome: AuditOutcome::Ok,
                latency_ms: Some(42),
                detail: None,
            })
            .await
            .expect("insert");

        let count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM audit_events")
            .fetch_one(&pool)
            .await
            .expect("count");
        assert_eq!(count.0, 1);
    }

    #[sqlx::test(migrations = "./migrations")]
    async fn record_rejects_invalid_outcome(pool: PgPool) {
        // El CHECK constraint de la tabla rechaza valores fuera del enum.
        let res = sqlx::query(
            r#"INSERT INTO audit_events (consumer_id, action, namespace, secret_path, outcome)
               VALUES ('x','y','z','p','invalid')"#,
        )
        .execute(&pool)
        .await;
        assert!(res.is_err(), "outcome inválido debe ser rechazado por CHECK");
    }
}
