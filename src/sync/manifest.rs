//! Manifest YAML del catálogo canónico de secretos (custodiado en Proton Pass).
//!
//! Schema:
//! ```yaml
//! version: 1
//! vault_id: alexendros-prod
//! secrets:
//!   - path: notion/token
//!     expected_keys: [token]
//!   - path: stripe/live/api_key
//!     expected_keys: [api_key]
//! ```
//!
//! El campo `expected_keys` lista las claves del JSON `data` del KV v2 que
//! el operador certifica que deben existir (audit de superficie · no
//! verificamos VALORES, sólo presencia · proteger valores requeriría
//! exfiltrar Vault → fuera de scope).

use std::path::Path;

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Manifest {
    pub version: u32,
    pub vault_id: String,
    pub secrets: Vec<ExpectedSecret>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExpectedSecret {
    /// Path relativo al KV mount (sin prefijo `data/`).
    pub path: String,
    /// Claves esperadas dentro del JSON `data` del KV v2.
    #[serde(default)]
    pub expected_keys: Vec<String>,
}

impl Manifest {
    pub fn load_from_file(path: &Path) -> Result<Self> {
        let raw = std::fs::read_to_string(path)
            .with_context(|| format!("leer manifest {}", path.display()))?;
        Self::from_yaml_str(&raw)
    }

    pub fn from_yaml_str(yaml: &str) -> Result<Self> {
        let m: Self = serde_yaml::from_str(yaml).context("parsear manifest YAML")?;
        if m.version != 1 {
            anyhow::bail!("manifest version {} no soportada (esperada 1)", m.version);
        }
        if m.vault_id.trim().is_empty() {
            anyhow::bail!("manifest.vault_id vacío");
        }
        for s in &m.secrets {
            if s.path.trim().is_empty() {
                anyhow::bail!("manifest secret con path vacío");
            }
            if s.path.starts_with('/') {
                anyhow::bail!("manifest secret path '{}' no debe empezar con '/'", s.path);
            }
        }
        Ok(m)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_yaml_minimo() {
        let y = r#"
version: 1
vault_id: test-vault
secrets:
  - path: notion/token
    expected_keys: [token]
  - path: stripe/api_key
"#;
        let m = Manifest::from_yaml_str(y).expect("parse");
        assert_eq!(m.vault_id, "test-vault");
        assert_eq!(m.secrets.len(), 2);
        assert_eq!(m.secrets[1].expected_keys.len(), 0);
    }

    #[test]
    fn rechaza_version_distinta() {
        let y = "version: 2\nvault_id: x\nsecrets: []";
        assert!(Manifest::from_yaml_str(y).is_err());
    }

    #[test]
    fn rechaza_vault_id_vacio() {
        let y = "version: 1\nvault_id: \"\"\nsecrets: []";
        assert!(Manifest::from_yaml_str(y).is_err());
    }

    #[test]
    fn rechaza_path_que_empieza_con_slash() {
        let y = r#"
version: 1
vault_id: x
secrets:
  - path: /no/leading/slash
"#;
        assert!(Manifest::from_yaml_str(y).is_err());
    }
}
