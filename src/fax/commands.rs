//! Comandos vía-fax (ADR-0007).
//!
//! El payload PGP-firmado contiene un YAML como:
//!
//! ```yaml
//! nonce: 6f5d3e26-cb04-4b78-bbac-3a3c8b4f0001
//! timestamp: 1748160000
//! command: invalidate
//! path: kv/proton/api_key
//! ```
//!
//! `command` discrimina la variante; el resto de campos depende del verbo.

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::{FaxError, FaxResult};

/// Sobre del comando: payload firmado YAML que llega al worker.
///
/// `nonce` + `timestamp` van validados por [`crate::security::ReplayCache`]
/// antes de despachar (ventana ±5 min, nonce único en cache de 5 min).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct FaxEnvelope {
    pub nonce: Uuid,
    pub timestamp: i64,
    #[serde(flatten)]
    pub command: FaxCommand,
}

/// Comando atómico. Cualquier nuevo verbo se añade aquí + handler en
/// [`super::dispatch::execute`].
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(tag = "command", rename_all = "kebab-case")]
pub enum FaxCommand {
    /// Invalida toda la cache local de Vault (`DashMap` en `VaultClient`).
    /// NO rota secretos en Vault — eso es `rotate-all` (PR7.1).
    InvalidateAll,
    /// Invalida la entrada cache de un path KV concreto. Útil cuando el
    /// operador acaba de rotar manualmente un secreto en Vault y quiere
    /// forzar a trenchpass a refetchear.
    Invalidate { path: String },
    /// Revoca un certificado emitido por Vault PKI por número de serie.
    /// El handler invocará `vaultrs::pki::cert::revoke` (PR7.1).
    Revoke { serial: String },
    /// Sella Vault. Operación destructiva: los siguientes requests fallan
    /// hasta unseal manual con las claves Shamir. Audit log obligatorio.
    SealVault,
}

/// Parsea un YAML armored sin firma → [`FaxEnvelope`].
pub fn parse_envelope(yaml: &[u8]) -> FaxResult<FaxEnvelope> {
    serde_yaml::from_slice(yaml).map_err(|e| FaxError::Command(format!("YAML parse: {e}")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_invalidate_all() {
        let yaml = b"nonce: 6f5d3e26-cb04-4b78-bbac-3a3c8b4f0001\n\
                     timestamp: 1748160000\n\
                     command: invalidate-all\n";
        let env = parse_envelope(yaml).expect("parse");
        assert_eq!(env.timestamp, 1748160000);
        assert_eq!(env.command, FaxCommand::InvalidateAll);
    }

    #[test]
    fn parse_invalidate_path() {
        let yaml = b"nonce: 6f5d3e26-cb04-4b78-bbac-3a3c8b4f0001\n\
                     timestamp: 1748160000\n\
                     command: invalidate\n\
                     path: kv/notion/api_key\n";
        let env = parse_envelope(yaml).expect("parse");
        assert_eq!(
            env.command,
            FaxCommand::Invalidate {
                path: "kv/notion/api_key".into()
            }
        );
    }

    #[test]
    fn parse_revoke() {
        let yaml = b"nonce: 6f5d3e26-cb04-4b78-bbac-3a3c8b4f0002\n\
                     timestamp: 1748160000\n\
                     command: revoke\n\
                     serial: '1a:2b:3c:4d'\n";
        let env = parse_envelope(yaml).expect("parse");
        assert_eq!(
            env.command,
            FaxCommand::Revoke {
                serial: "1a:2b:3c:4d".into()
            }
        );
    }

    #[test]
    fn parse_seal_vault() {
        let yaml = b"nonce: 6f5d3e26-cb04-4b78-bbac-3a3c8b4f0003\n\
                     timestamp: 1748160000\n\
                     command: seal-vault\n";
        let env = parse_envelope(yaml).expect("parse");
        assert_eq!(env.command, FaxCommand::SealVault);
    }

    #[test]
    fn parse_unknown_command_is_error() {
        let yaml = b"nonce: 6f5d3e26-cb04-4b78-bbac-3a3c8b4f0004\n\
                     timestamp: 1748160000\n\
                     command: format-disks\n";
        assert!(parse_envelope(yaml).is_err());
    }

    #[test]
    fn parse_missing_serial_is_error() {
        let yaml = b"nonce: 6f5d3e26-cb04-4b78-bbac-3a3c8b4f0005\n\
                     timestamp: 1748160000\n\
                     command: revoke\n";
        assert!(parse_envelope(yaml).is_err());
    }
}
