//! Cliente Vault PKI · emisión de certificados leaf con TTL acotado.
//!
//! Devuelve PEMs en memoria (nunca tocan disco). El llamante construye
//! `RustlsConfig::from_pem(...)` directamente con los `Vec<u8>`.

use std::time::Duration;

use tracing::instrument;
use vaultrs::api::pki::requests::GenerateCertificateRequest;
use vaultrs::pki::cert;

use super::client::VaultClient;
use crate::error::{Error, Result};

/// Bundle de certificado emitido por Vault PKI.
#[derive(Debug, Clone)]
pub struct CertBundle {
    /// PEM del leaf (sin la cadena · primer cert).
    pub certificate: String,
    /// Cadena de CAs intermedias (sin root). Orden: emisor inmediato primero.
    pub ca_chain: Vec<String>,
    /// PEM de la clave privada (PKCS#8 o RSA según role).
    pub private_key: String,
    /// CA emisora directa (alias del primer elemento del `ca_chain` o `issuing_ca`).
    pub issuing_ca: String,
    /// Serial hex separado por `:` (uso: audit + revoke).
    pub serial_number: String,
    /// TTL real concedido por Vault (≤ ttl solicitado, ≤ role.max_ttl).
    pub ttl: Duration,
}

impl CertBundle {
    /// Concatena leaf + CAs intermedias en orden rustls (`leaf, intermediate_1, ...`).
    /// NO incluye la trust root — eso lo verifica el cliente.
    pub fn fullchain_pem(&self) -> String {
        let mut out = String::with_capacity(self.certificate.len() + 1024);
        out.push_str(self.certificate.trim_end());
        out.push('\n');
        for ca in &self.ca_chain {
            out.push_str(ca.trim_end());
            out.push('\n');
        }
        out
    }
}

impl VaultClient {
    /// Solicita a Vault PKI un cert leaf para `role` con `common_name` y `alt_names`.
    /// `ttl` se solicita en segundos; Vault aplica el mínimo entre éste y `role.max_ttl`.
    #[instrument(skip(self), fields(mount, role, cn = %common_name))]
    pub async fn issue_cert(
        &self,
        mount: &str,
        role: &str,
        common_name: &str,
        alt_names: &[String],
        ttl: Duration,
    ) -> Result<CertBundle> {
        let mut opts = GenerateCertificateRequest::builder();
        opts.common_name(common_name.to_string())
            .ttl(format!("{}s", ttl.as_secs()));
        if !alt_names.is_empty() {
            opts.alt_names(alt_names.join(","));
        }

        let resp = cert::generate(self.raw_ref(), mount, role, Some(&mut opts))
            .await
            .map_err(|e| Error::Vault(format!("pki/{mount}/issue/{role}: {e}")))?;

        let granted_ttl = resp
            .expiration
            .map(|exp| {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                Duration::from_secs(exp.saturating_sub(now))
            })
            .unwrap_or(ttl);

        Ok(CertBundle {
            certificate: resp.certificate,
            ca_chain: resp.ca_chain.unwrap_or_default(),
            private_key: resp.private_key,
            issuing_ca: resp.issuing_ca,
            serial_number: resp.serial_number,
            ttl: granted_ttl,
        })
    }

    /// Lee el `ca_chain` del mount PKI (root + intermediates emitidos por Vault).
    /// Se usa como trust anchors para `WebPkiClientVerifier`.
    #[instrument(skip(self), fields(mount))]
    pub async fn pki_ca_chain(&self, mount: &str) -> Result<Vec<String>> {
        // Vault expone la cadena entera (root + intermediates) en el "serial"
        // mágico `ca_chain`. La respuesta `certificate` viene como múltiples
        // bloques PEM concatenados.
        let resp = cert::read(self.raw_ref(), mount, "ca_chain")
            .await
            .map_err(|e| Error::Vault(format!("pki/{mount}/cert/ca_chain: {e}")))?;
        if resp.certificate.trim().is_empty() {
            return Err(Error::Vault(format!(
                "pki/{mount}/cert/ca_chain devolvió cadena vacía"
            )));
        }
        Ok(split_pem_blocks(&resp.certificate))
    }
}

fn split_pem_blocks(pem: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut current = String::new();
    let mut in_block = false;
    for line in pem.lines() {
        if line.starts_with("-----BEGIN") {
            in_block = true;
            current.clear();
        }
        if in_block {
            current.push_str(line);
            current.push('\n');
        }
        if line.starts_with("-----END") {
            out.push(std::mem::take(&mut current));
            in_block = false;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_pem_blocks_separa_dos_certs() {
        let raw = "\
-----BEGIN CERTIFICATE-----
AAAA
-----END CERTIFICATE-----
-----BEGIN CERTIFICATE-----
BBBB
-----END CERTIFICATE-----
";
        let blocks = split_pem_blocks(raw);
        assert_eq!(blocks.len(), 2);
        assert!(blocks[0].contains("AAAA"));
        assert!(blocks[1].contains("BBBB"));
    }

    #[test]
    fn fullchain_pem_orden_leaf_first() {
        let bundle = CertBundle {
            certificate: "LEAF".into(),
            ca_chain: vec!["INTER1".into(), "INTER2".into()],
            private_key: "KEY".into(),
            issuing_ca: "INTER1".into(),
            serial_number: "00".into(),
            ttl: Duration::from_secs(3600),
        };
        let chain = bundle.fullchain_pem();
        let leaf_pos = chain.find("LEAF").unwrap();
        let inter1_pos = chain.find("INTER1").unwrap();
        let inter2_pos = chain.find("INTER2").unwrap();
        assert!(leaf_pos < inter1_pos);
        assert!(inter1_pos < inter2_pos);
    }
}
