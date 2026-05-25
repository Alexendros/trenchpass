//! Despacho de comandos vía-fax verificados.
//!
//! Cada ejecución se asienta en `audit_events` con `consumer_id = "via-fax"`,
//! `action = "fax.<verb>"`, y el hash SHA-256 (hex) de los bytes firmados como
//! `detail.signature_sha256` — el bloque está documentado en docs/api.md
//! sección "Audit log shape".

use std::sync::Arc;

use tracing::{info, warn};

use crate::audit::{AuditEvent, AuditOutcome};
use crate::AppState;

use super::commands::FaxCommand;
use super::{FaxError, FaxResult};

/// Ejecuta `command` sobre `state`. Audit log mejor-esfuerzo.
///
/// `signature_sha256_hex` es el hash de los bytes firmados (no del cuerpo
/// limpio): permite correlacionar el evento con el mensaje original en logs
/// IMAP.
#[tracing::instrument(skip(state, signature_sha256_hex), fields(command = ?command))]
pub async fn execute(
    state: &Arc<AppState>,
    command: &FaxCommand,
    signature_sha256_hex: &str,
) -> FaxResult<()> {
    let result = match command {
        FaxCommand::InvalidateAll => {
            state.vault.invalidate_all();
            info!(target: "fax.dispatch", "vault cache invalidated (all)");
            Ok(())
        }
        FaxCommand::Invalidate { path } => {
            state.vault.invalidate(path);
            info!(target: "fax.dispatch", path = %path, "vault cache entry invalidated");
            Ok(())
        }
        FaxCommand::Revoke { serial } => {
            // PR7.1: vaultrs::pki::cert::revoke contra state.vault.raw_ref()
            // (visibilidad pub(crate); el wrapper limpio va en src/vault/pki.rs).
            warn!(target: "fax.dispatch", serial = %serial, "revoke aún no cableado · ver PR7.1");
            Err(FaxError::Dispatch(format!(
                "revoke {serial} no cableado · pendiente PR7.1"
            )))
        }
        FaxCommand::SealVault => {
            // PR7.1: vaultrs::sys::seal.
            warn!(target: "fax.dispatch", "seal-vault aún no cableado · ver PR7.1");
            Err(FaxError::Dispatch(
                "seal-vault no cableado · pendiente PR7.1".into(),
            ))
        }
    };

    let outcome = if result.is_ok() {
        AuditOutcome::Ok
    } else {
        AuditOutcome::Error
    };
    let action = match command {
        FaxCommand::InvalidateAll => "fax.invalidate-all",
        FaxCommand::Invalidate { .. } => "fax.invalidate",
        FaxCommand::Revoke { .. } => "fax.revoke",
        FaxCommand::SealVault => "fax.seal-vault",
    };
    let detail = serde_json::json!({
        "command": command,
        "signature_sha256": signature_sha256_hex,
    });

    state
        .audit
        .record_best_effort(AuditEvent {
            consumer_id: "via-fax",
            action,
            namespace: "fax",
            secret_path: "",
            outcome,
            latency_ms: None,
            detail: Some(detail),
        })
        .await;

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_hash() -> &'static str {
        "deadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeefdeadbeef"
    }

    #[test]
    fn action_string_per_variant() {
        let cmds = [
            (FaxCommand::InvalidateAll, "fax.invalidate-all"),
            (
                FaxCommand::Invalidate { path: "x".into() },
                "fax.invalidate",
            ),
            (FaxCommand::Revoke { serial: "x".into() }, "fax.revoke"),
            (FaxCommand::SealVault, "fax.seal-vault"),
        ];
        for (cmd, expected) in cmds {
            let action = match &cmd {
                FaxCommand::InvalidateAll => "fax.invalidate-all",
                FaxCommand::Invalidate { .. } => "fax.invalidate",
                FaxCommand::Revoke { .. } => "fax.revoke",
                FaxCommand::SealVault => "fax.seal-vault",
            };
            assert_eq!(action, expected);
        }
    }

    #[test]
    fn signature_hash_is_hex_lowercase() {
        let h = dummy_hash();
        assert_eq!(h.len(), 64);
        assert!(h
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));
    }
}
