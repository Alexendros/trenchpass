//! Sync worker · drift detection entre el manifest canónico (Proton Pass)
//! y la cache runtime (Vault KV v2).
//!
//! Estrategia (PR4):
//! - **Source canónico**: manifest YAML versionado del operador (custodiado en
//!   Proton Pass · file `proton-pass-manifest.yaml`). Define los secretos que
//!   DEBEN existir y sus claves esperadas.
//! - **Source runtime**: Vault KV v2 mount (lo que el gateway ve realmente).
//! - **Worker**: tokio task que cada `drift_interval` hace `detect_drift` y
//!   escribe un audit event por cada anomalía (best-effort) + log estructurado.
//!
//! Integración con Proton Pass CLI nativa queda para PR4.1 (cuando esté
//! definida la interfaz exacta del binario `protonpass-cli`).

pub mod drift;
pub mod manifest;
pub mod source;
pub mod worker;

pub use drift::{detect_drift, DriftEvent};
pub use manifest::{ExpectedSecret, Manifest};
pub use source::{ManifestSource, SecretSource};
pub use worker::spawn_drift_worker;
