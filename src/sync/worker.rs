//! Worker periódico que ejecuta `detect_drift` y emite eventos de audit.

use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::task::JoinHandle;
use tracing::{debug, error, info, warn};

use super::drift::{detect_drift, DriftEvent};
use super::source::SecretSource;
use crate::audit::{AuditEvent, AuditOutcome, AuditStore};
use crate::vault::VaultClient;

/// Action emitido al audit log para cada drift detectado.
const ACTION_DRIFT: &str = "sync.drift_detected";
/// Consumer sintético que firma los eventos del worker.
const CONSUMER_SYSTEM: &str = "system.sync";
/// Namespace que cubre los eventos del sync worker.
const NAMESPACE_SYNC: &str = "_sync";

/// Spawn de la tarea periódica de drift detection.
/// Si `interval == 0`, NO arranca el worker (modo desactivado).
pub fn spawn_drift_worker(
    interval: Duration,
    source: Arc<dyn SecretSource>,
    vault: VaultClient,
    audit: AuditStore,
    kv_mount: String,
) -> Option<JoinHandle<()>> {
    if interval.is_zero() {
        info!(target: "sync.worker", "drift worker DESACTIVADO (interval=0)");
        return None;
    }
    info!(
        target: "sync.worker",
        interval_secs = interval.as_secs(),
        kv_mount = %kv_mount,
        "drift worker iniciado"
    );
    Some(tokio::spawn(async move {
        // Primer tick inmediato + jitter pequeño para evitar thundering herd
        // cuando se relanzan varios gateways a la vez.
        tokio::time::sleep(Duration::from_secs(5)).await;
        loop {
            run_one_pass(&*source, &vault, &audit, &kv_mount).await;
            tokio::time::sleep(interval).await;
        }
    }))
}

async fn run_one_pass(
    source: &dyn SecretSource,
    vault: &VaultClient,
    audit: &AuditStore,
    kv_mount: &str,
) {
    let started = Instant::now();
    match detect_drift(source, vault, kv_mount).await {
        Ok(events) => {
            let elapsed = started.elapsed();
            if events.is_empty() {
                debug!(
                    target: "sync.worker",
                    elapsed_ms = elapsed.as_millis() as u64,
                    "pasada limpia · 0 drifts"
                );
                return;
            }
            warn!(
                target: "sync.worker",
                drift_count = events.len(),
                elapsed_ms = elapsed.as_millis() as u64,
                "drifts detectados · emitiendo audit events"
            );
            for ev in &events {
                emit_audit(audit, ev).await;
            }
        }
        Err(e) => {
            error!(
                target: "sync.worker",
                error = %e,
                "detect_drift falló · saltamos pasada"
            );
        }
    }
}

async fn emit_audit(audit: &AuditStore, event: &DriftEvent) {
    let detail = serde_json::to_value(event).unwrap_or(serde_json::Value::Null);
    let path_owned = event.path().to_string();
    let kind_owned = event.kind().to_string();
    let evt = AuditEvent {
        consumer_id: CONSUMER_SYSTEM,
        action: ACTION_DRIFT,
        namespace: NAMESPACE_SYNC,
        secret_path: &path_owned,
        outcome: AuditOutcome::Denied,
        latency_ms: None,
        detail: Some(detail),
    };
    audit.record_best_effort(evt).await;
    warn!(
        target: "sync.drift",
        kind = %kind_owned,
        path = %path_owned,
        "drift"
    );
}

// Tests del worker viven en `tests/it_sync.rs` (gated `it-postgres`) porque
// requieren un AuditStore real. La lógica de drift detection se testa
// unitariamente en `drift.rs` y `manifest.rs`.
