//! Trait para fuentes canónicas de secretos esperados.
//!
//! En PR4 sólo proveemos [`ManifestSource`] (lectura del YAML).
//! PR4.1 añadirá `ProtonPassCliSource` cuando el binario `protonpass-cli`
//! tenga interfaz estable.

use anyhow::Result;
use async_trait::async_trait;

use super::manifest::{ExpectedSecret, Manifest};

#[async_trait]
pub trait SecretSource: Send + Sync {
    /// Devuelve el set de secretos esperados (canónicos).
    /// Debe ser cheap-to-call · llamado en cada tick del worker.
    async fn list_expected(&self) -> Result<Vec<ExpectedSecret>>;
}

/// Fuente basada en manifest YAML pre-cargado en memoria.
pub struct ManifestSource {
    manifest: Manifest,
}

impl ManifestSource {
    pub fn new(manifest: Manifest) -> Self {
        Self { manifest }
    }

    pub fn vault_id(&self) -> &str {
        &self.manifest.vault_id
    }
}

#[async_trait]
impl SecretSource for ManifestSource {
    async fn list_expected(&self) -> Result<Vec<ExpectedSecret>> {
        Ok(self.manifest.secrets.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn manifest_source_round_trip() {
        let m = Manifest {
            version: 1,
            vault_id: "v".into(),
            secrets: vec![ExpectedSecret {
                path: "a/b".into(),
                expected_keys: vec!["k".into()],
            }],
        };
        let src = ManifestSource::new(m);
        let list = src.list_expected().await.unwrap();
        assert_eq!(list.len(), 1);
        assert_eq!(list[0].path, "a/b");
    }
}
