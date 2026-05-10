//! Detección de drift entre el set canónico (manifest) y el runtime (Vault KV v2).
//!
//! Tres tipos de evento:
//! - [`DriftEvent::MissingFromVault`]: el manifest dice que `path` debe existir,
//!   pero Vault no lo tiene (o falla al leerlo).
//! - [`DriftEvent::ExtraInVault`]: el path existe en Vault pero el manifest no lo
//!   declara (puede ser legítimo · señaliza para audit, no rompe).
//! - [`DriftEvent::KeyMismatch`]: el path existe en ambos pero las claves del JSON
//!   `data` no coinciden con `expected_keys`.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use tracing::warn;

use super::source::SecretSource;
use crate::vault::VaultClient;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum DriftEvent {
    MissingFromVault {
        path: String,
        expected_keys: Vec<String>,
    },
    ExtraInVault {
        path: String,
    },
    KeyMismatch {
        path: String,
        expected: Vec<String>,
        actual: Vec<String>,
        only_expected: Vec<String>,
        only_actual: Vec<String>,
    },
}

impl DriftEvent {
    pub fn path(&self) -> &str {
        match self {
            Self::MissingFromVault { path, .. }
            | Self::ExtraInVault { path }
            | Self::KeyMismatch { path, .. } => path,
        }
    }

    pub fn kind(&self) -> &'static str {
        match self {
            Self::MissingFromVault { .. } => "missing_from_vault",
            Self::ExtraInVault { .. } => "extra_in_vault",
            Self::KeyMismatch { .. } => "key_mismatch",
        }
    }
}

/// Compara el manifest canónico contra Vault y devuelve la lista de drifts.
///
/// `kv_mount` es el mount KV v2 (e.g. `secret`). Recorre recursivamente.
/// Errores de red de Vault se propagan; un secreto individual que falla la
/// lectura se reporta como `MissingFromVault` (no detiene la pasada completa).
pub async fn detect_drift<S: SecretSource + ?Sized>(
    source: &S,
    vault: &VaultClient,
    kv_mount: &str,
) -> Result<Vec<DriftEvent>> {
    let expected = source.list_expected().await?;
    let actual_paths = vault.list_kv_paths(kv_mount, "").await?;

    let expected_set: std::collections::HashSet<&str> =
        expected.iter().map(|s| s.path.as_str()).collect();
    let actual_set: std::collections::HashSet<&str> =
        actual_paths.iter().map(|s| s.as_str()).collect();

    let mut events = Vec::new();

    // 1) Missing: en manifest pero no en Vault list.
    // 2) KeyMismatch: presentes en ambos, leemos el secret y comparamos keys.
    for exp in &expected {
        if !actual_set.contains(exp.path.as_str()) {
            events.push(DriftEvent::MissingFromVault {
                path: exp.path.clone(),
                expected_keys: exp.expected_keys.clone(),
            });
            continue;
        }
        if exp.expected_keys.is_empty() {
            continue;
        }
        match vault.secret(&exp.path).await {
            Ok(secret) => {
                let actual_keys = extract_data_keys(&secret.data);
                let (only_expected, only_actual) = diff_keys(&exp.expected_keys, &actual_keys);
                if !only_expected.is_empty() || !only_actual.is_empty() {
                    events.push(DriftEvent::KeyMismatch {
                        path: exp.path.clone(),
                        expected: exp.expected_keys.clone(),
                        actual: actual_keys,
                        only_expected,
                        only_actual,
                    });
                }
            }
            Err(e) => {
                warn!(
                    target: "sync.drift",
                    path = %exp.path,
                    error = %e,
                    "no se pudo leer secret declarado en manifest · reportado como Missing"
                );
                events.push(DriftEvent::MissingFromVault {
                    path: exp.path.clone(),
                    expected_keys: exp.expected_keys.clone(),
                });
            }
        }
    }

    // 3) Extra: en Vault pero no en manifest.
    for actual in &actual_paths {
        if !expected_set.contains(actual.as_str()) {
            events.push(DriftEvent::ExtraInVault {
                path: actual.clone(),
            });
        }
    }

    Ok(events)
}

/// Extrae las claves del campo `data` del JSON KV v2.
/// KV v2 envuelve los datos: `{"data": {"data": {key: value, ...}, "metadata": ...}}`.
/// Si falla el unwrap, devuelve las claves del nivel superior.
fn extract_data_keys(value: &serde_json::Value) -> Vec<String> {
    if let Some(data_outer) = value.get("data") {
        if let Some(inner) = data_outer.get("data") {
            if let Some(obj) = inner.as_object() {
                return obj.keys().cloned().collect();
            }
        }
        if let Some(obj) = data_outer.as_object() {
            return obj.keys().cloned().collect();
        }
    }
    if let Some(obj) = value.as_object() {
        return obj.keys().cloned().collect();
    }
    Vec::new()
}

fn diff_keys(expected: &[String], actual: &[String]) -> (Vec<String>, Vec<String>) {
    let exp: std::collections::HashSet<&str> = expected.iter().map(|s| s.as_str()).collect();
    let act: std::collections::HashSet<&str> = actual.iter().map(|s| s.as_str()).collect();
    let only_expected: Vec<String> = exp.difference(&act).map(|s| s.to_string()).collect();
    let only_actual: Vec<String> = act.difference(&exp).map(|s| s.to_string()).collect();
    let mut oe = only_expected;
    let mut oa = only_actual;
    oe.sort();
    oa.sort();
    (oe, oa)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn extract_data_keys_kv2_envelope() {
        let v = json!({
            "data": {
                "data": {"token": "x", "scope": "y"},
                "metadata": {"version": 1}
            }
        });
        let keys = extract_data_keys(&v);
        assert_eq!(keys.len(), 2);
        assert!(keys.contains(&"token".to_string()));
        assert!(keys.contains(&"scope".to_string()));
    }

    #[test]
    fn extract_data_keys_flat_data() {
        let v = json!({"data": {"k": "v"}});
        let keys = extract_data_keys(&v);
        assert_eq!(keys, vec!["k"]);
    }

    #[test]
    fn diff_keys_correcto() {
        let (oe, oa) = diff_keys(
            &["a".into(), "b".into(), "c".into()],
            &["b".into(), "c".into(), "d".into()],
        );
        assert_eq!(oe, vec!["a"]);
        assert_eq!(oa, vec!["d"]);
    }

    #[test]
    fn drift_event_kind_y_path() {
        let m = DriftEvent::MissingFromVault {
            path: "x/y".into(),
            expected_keys: vec![],
        };
        assert_eq!(m.kind(), "missing_from_vault");
        assert_eq!(m.path(), "x/y");
    }
}
