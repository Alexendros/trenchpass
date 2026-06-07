//! Rutas de operación `/admin/*` · invalidación de cache y refresh de trust.
//!
//! Necesarias para el cutover de Controlink: durante el dual-run el operador
//! rota/revoca credenciales y necesita que el gateway deje de servir el valor
//! cacheado sin esperar el TTL de 60 s. Antes de PR6 esto sólo era accesible por
//! vía-fax (asíncrono, minutos de latencia).
//!
//! Protección: el consumidor debe portar scope `admin:*` (o `*`). Se reutiliza
//! [`scope::check`] con ids sintéticos `admin.invalidate` / `admin.refresh_crl`.

use std::sync::Arc;

use axum::{extract::State, Extension, Json};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::audit::{AuditEvent, AuditOutcome};
use crate::auth::{scope, Consumer};
use crate::error::{Error, Result};
use crate::AppState;

#[derive(Debug, Deserialize)]
pub struct InvalidateBody {
    /// Path KV v2 relativo al mount (ej. `consumers/stripe-fetcher`). Si se
    /// omite, se vacía el cache completo.
    #[serde(default)]
    pub path: Option<String>,
}

/// `POST /admin/invalidate` · expulsa una entrada (o todas) del cache de Vault.
pub async fn invalidate(
    State(state): State<Arc<AppState>>,
    Extension(consumer): Extension<Consumer>,
    Json(body): Json<InvalidateBody>,
) -> Result<Json<Value>> {
    require_admin(&consumer, "admin.invalidate")?;

    let scope_invalidated = match body.path.as_deref() {
        Some(p) if !p.is_empty() => {
            state.vault.invalidate(p);
            p.to_string()
        }
        _ => {
            state.vault.invalidate_all();
            "*".to_string()
        }
    };

    audit_admin(&state, &consumer, "admin.invalidate", &scope_invalidated).await;
    Ok(Json(json!({ "invalidated": scope_invalidated })))
}

/// `POST /admin/refresh-crl` · fuerza re-lectura de material de confianza.
///
/// El reload de trust anchors del listener mTLS corre en su propio loop
/// (`transport::mtls::spawn_refresh_loop`); este endpoint vacía el cache de
/// secretos para que CRL/consumer-secrets recién rotados se re-lean de Vault de
/// inmediato en vez de esperar el TTL.
pub async fn refresh_crl(
    State(state): State<Arc<AppState>>,
    Extension(consumer): Extension<Consumer>,
) -> Result<Json<Value>> {
    require_admin(&consumer, "admin.refresh_crl")?;
    state.vault.invalidate_all();
    audit_admin(&state, &consumer, "admin.refresh_crl", "*").await;
    Ok(Json(
        json!({ "status": "cache flushed", "note": "trust anchors reload via PKI refresh loop" }),
    ))
}

fn require_admin(consumer: &Consumer, action: &str) -> Result<()> {
    scope::check(action, &consumer.scopes).map_err(|_| Error::ScopeViolation {
        required: "admin:*".to_string(),
        granted: consumer.scopes.clone(),
    })
}

async fn audit_admin(state: &AppState, consumer: &Consumer, action: &'static str, target: &str) {
    state
        .audit
        .record_best_effort(AuditEvent {
            consumer_id: &consumer.id,
            action,
            namespace: "admin",
            secret_path: target,
            outcome: AuditOutcome::Ok,
            latency_ms: None,
            detail: None,
        })
        .await;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn consumer_with(scopes: &[&str]) -> Consumer {
        Consumer {
            id: "op".into(),
            scopes: scopes.iter().map(|s| s.to_string()).collect(),
            ttl_secs: None,
        }
    }

    #[test]
    fn admin_scope_grants() {
        require_admin(&consumer_with(&["admin:*"]), "admin.invalidate").unwrap();
        require_admin(&consumer_with(&["*"]), "admin.refresh_crl").unwrap();
    }

    #[test]
    fn non_admin_scope_denied() {
        // Un fetcher de Stripe no puede invalidar cache ni refrescar CRL.
        let err = require_admin(&consumer_with(&["stripe:*"]), "admin.invalidate").unwrap_err();
        match err {
            Error::ScopeViolation { required, .. } => assert_eq!(required, "admin:*"),
            other => panic!("se esperaba ScopeViolation, fue {other:?}"),
        }
    }
}
