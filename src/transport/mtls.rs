//! mTLS sobre rustls 0.23 (aws-lc-rs) con axum-server 0.7.
//!
//! Dos modos:
//! - [`TlsMode::Static`]: PEMs en disco · break-glass / dev.
//! - [`TlsMode::VaultPki`]: leaf emitido por Vault PKI con TTL ≤ 7 d, refresh
//!   automático en bucle `tokio::spawn`. La rotación es in-process sin
//!   downtime gracias a [`RustlsConfig::reload_from_pem`] de axum-server.
//!
//! Cadena rustls (orden estricto): `[leaf, intermediate_1, ...]` (sin root).
//! Trust anchors para `WebPkiClientVerifier`: el `ca_chain` del mount Vault PKI
//! (root + intermediate emitidos por Vault). En modo Static, el `client_ca` PEM.

use std::sync::Arc;
use std::time::Duration;

use anyhow::{anyhow, bail, Context, Result};
use axum_server::tls_rustls::RustlsConfig;
use rustls::server::WebPkiClientVerifier;
use rustls::RootCertStore;
use rustls_pki_types::CertificateDer;
use tracing::{error, info};

use crate::config::{TlsConfig, TlsMode};
use crate::vault::{CertBundle, VaultClient};

/// Estado vivo del transport TLS devuelto por [`build`].
pub struct TlsHandle {
    pub config: RustlsConfig,
    /// Sólo presente en modo VaultPki para alimentar el refresh loop.
    pub initial_bundle: Option<CertBundle>,
}

/// Construye `RustlsConfig` listo para `axum_server::bind_rustls`.
/// Llama internamente a [`crate::init_crypto`].
pub async fn build(
    cfg: &TlsConfig,
    vault: Option<&VaultClient>,
    pki_mount: Option<&str>,
) -> Result<TlsHandle> {
    crate::init_crypto();

    match cfg.mode {
        TlsMode::Off => bail!("transport::mtls::build invocado con TlsMode::Off"),
        TlsMode::Static => build_static(cfg).await,
        TlsMode::VaultPki => {
            let vault = vault.ok_or_else(|| anyhow!("VaultPki requiere VaultClient"))?;
            let mount = pki_mount.ok_or_else(|| anyhow!("VaultPki requiere pki_mount"))?;
            build_vault_pki(cfg, vault, mount).await
        }
    }
}

async fn build_static(cfg: &TlsConfig) -> Result<TlsHandle> {
    let cert = cfg
        .cert
        .as_ref()
        .ok_or_else(|| anyhow!("TlsMode::Static requiere TRENCHPASS_TLS_CERT"))?;
    let key = cfg
        .key
        .as_ref()
        .ok_or_else(|| anyhow!("TlsMode::Static requiere TRENCHPASS_TLS_KEY"))?;

    let cert_bytes = tokio::fs::read(cert)
        .await
        .with_context(|| format!("leer cert {}", cert.display()))?;
    let key_bytes = tokio::fs::read(key)
        .await
        .with_context(|| format!("leer key {}", key.display()))?;

    let server_config = match cfg.client_ca.as_ref() {
        Some(client_ca) => {
            let ca_bytes = tokio::fs::read(client_ca)
                .await
                .with_context(|| format!("leer client_ca {}", client_ca.display()))?;
            build_server_config(&cert_bytes, &key_bytes, Some(&ca_bytes), cfg.mtls_required)?
        }
        None => {
            if cfg.mtls_required {
                bail!("mtls_required=true requiere TRENCHPASS_TLS_CLIENT_CA");
            }
            build_server_config(&cert_bytes, &key_bytes, None, false)?
        }
    };

    let config = RustlsConfig::from_config(Arc::new(server_config));
    info!(target: "trenchpass.tls", mode = "static", "TLS listo (PEMs estáticos)");
    Ok(TlsHandle {
        config,
        initial_bundle: None,
    })
}

async fn build_vault_pki(
    cfg: &TlsConfig,
    vault: &VaultClient,
    pki_mount: &str,
) -> Result<TlsHandle> {
    info!(
        target: "trenchpass.tls",
        mount = pki_mount,
        role = %cfg.pki_role,
        cn = %cfg.pki_common_name,
        ttl_secs = cfg.pki_cert_ttl.as_secs(),
        "emitiendo leaf cert vía Vault PKI"
    );

    let bundle = vault
        .issue_cert(
            pki_mount,
            &cfg.pki_role,
            &cfg.pki_common_name,
            &cfg.pki_alt_names,
            cfg.pki_cert_ttl,
        )
        .await
        .context("Vault PKI issue_cert (bootstrap)")?;

    let ca_chain_pem = vault
        .pki_ca_chain(pki_mount)
        .await
        .context("Vault PKI ca_chain (trust anchors)")?
        .join("\n");

    let fullchain = bundle.fullchain_pem();
    let server_config = build_server_config(
        fullchain.as_bytes(),
        bundle.private_key.as_bytes(),
        Some(ca_chain_pem.as_bytes()),
        cfg.mtls_required,
    )?;

    let config = RustlsConfig::from_config(Arc::new(server_config));
    info!(
        target: "trenchpass.tls",
        mode = "vault_pki",
        serial = %bundle.serial_number,
        ttl_secs = bundle.ttl.as_secs(),
        "TLS listo (Vault PKI)"
    );

    Ok(TlsHandle {
        config,
        initial_bundle: Some(bundle),
    })
}

/// Construye `rustls::ServerConfig` con `WebPkiClientVerifier` opcional.
fn build_server_config(
    cert_chain_pem: &[u8],
    key_pem: &[u8],
    client_ca_pem: Option<&[u8]>,
    require_client_cert: bool,
) -> Result<rustls::ServerConfig> {
    let cert_chain = parse_cert_chain(cert_chain_pem).context("parsear cert chain")?;
    let key = parse_private_key(key_pem).context("parsear private key")?;

    let builder = rustls::ServerConfig::builder();

    let server_config = match client_ca_pem {
        Some(ca_pem) => {
            let mut roots = RootCertStore::empty();
            for der in parse_cert_chain(ca_pem).context("parsear client_ca PEMs")? {
                roots.add(der).context("añadir trust anchor")?;
            }
            let mut verifier_builder = WebPkiClientVerifier::builder(Arc::new(roots));
            if !require_client_cert {
                verifier_builder = verifier_builder.allow_unauthenticated();
            }
            let verifier = verifier_builder.build().context("WebPkiClientVerifier")?;
            builder
                .with_client_cert_verifier(verifier)
                .with_single_cert(cert_chain, key)
                .context("ServerConfig with_single_cert (mTLS)")?
        }
        None => builder
            .with_no_client_auth()
            .with_single_cert(cert_chain, key)
            .context("ServerConfig with_single_cert (TLS)")?,
    };

    Ok(server_config)
}

fn parse_cert_chain(pem: &[u8]) -> Result<Vec<CertificateDer<'static>>> {
    let mut reader = std::io::BufReader::new(pem);
    let chain: std::io::Result<Vec<_>> = rustls_pemfile::certs(&mut reader).collect();
    let chain = chain.context("rustls_pemfile::certs")?;
    if chain.is_empty() {
        bail!("PEM no contiene certificados");
    }
    Ok(chain)
}

fn parse_private_key(pem: &[u8]) -> Result<rustls_pki_types::PrivateKeyDer<'static>> {
    let mut reader = std::io::BufReader::new(pem);
    rustls_pemfile::private_key(&mut reader)
        .context("rustls_pemfile::private_key")?
        .ok_or_else(|| anyhow!("PEM no contiene private key"))
}

/// Backoff entre reintentos cuando Vault PKI no responde.
const REFRESH_RETRY_BACKOFF: Duration = Duration::from_secs(30);

/// Spawn de tarea de refresh para modo VaultPki.
/// Re-emite cuando se consume `refresh_percent%` del TTL, con jitter ±N%.
/// Política de fallo: tras un `Err`, la siguiente iteración duerme sólo
/// `REFRESH_RETRY_BACKOFF` (no el ttl×refresh_pct/100 normal) — así no
/// caemos en `30 s + 3.5 d` durante una caída prolongada de Vault.
pub fn spawn_refresh_loop(
    config: RustlsConfig,
    vault: VaultClient,
    pki_mount: String,
    cfg: TlsConfig,
    initial: CertBundle,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut current = initial;
        let mut last_failed = false;
        loop {
            let sleep_for = if last_failed {
                REFRESH_RETRY_BACKOFF
            } else {
                compute_refresh_delay(
                    current.ttl,
                    cfg.pki_refresh_percent,
                    cfg.pki_refresh_jitter_percent,
                )
            };
            info!(
                target: "trenchpass.tls.refresh",
                serial = %current.serial_number,
                sleep_secs = sleep_for.as_secs(),
                retry = last_failed,
                "scheduling next cert refresh"
            );
            tokio::time::sleep(sleep_for).await;

            match issue_and_reload(&config, &vault, &pki_mount, &cfg).await {
                Ok(new_bundle) => {
                    info!(
                        target: "trenchpass.tls.refresh",
                        old_serial = %current.serial_number,
                        new_serial = %new_bundle.serial_number,
                        new_ttl_secs = new_bundle.ttl.as_secs(),
                        "cert reloaded"
                    );
                    current = new_bundle;
                    last_failed = false;
                }
                Err(e) => {
                    error!(
                        target: "trenchpass.tls.refresh",
                        error = %e,
                        backoff_secs = REFRESH_RETRY_BACKOFF.as_secs(),
                        "refresh falló · reintentando con backoff corto"
                    );
                    last_failed = true;
                }
            }
        }
    })
}

/// Reemite leaf+ca_chain y reconstruye `ServerConfig` completo.
///
/// Decisión: SIEMPRE reconstruimos (no usamos `reload_from_pem`) en modo
/// `vault_pki` porque Vault puede rotar la CA intermedia entre refreshes;
/// si reusáramos el verifier viejo, los certs cliente firmados por el nuevo
/// emisor serían rechazados pese a ser válidos. La reemisión del verifier
/// es barata y consistente.
async fn issue_and_reload(
    config: &RustlsConfig,
    vault: &VaultClient,
    pki_mount: &str,
    cfg: &TlsConfig,
) -> Result<CertBundle> {
    // Defensive: garantiza que `ServerConfig::builder()` tenga provider aunque
    // este path se invoque antes de `build()` (e.g. tests, refactor futuro).
    crate::init_crypto();

    let bundle = vault
        .issue_cert(
            pki_mount,
            &cfg.pki_role,
            &cfg.pki_common_name,
            &cfg.pki_alt_names,
            cfg.pki_cert_ttl,
        )
        .await
        .context("Vault PKI issue_cert (refresh)")?;

    let ca_chain_pem = vault
        .pki_ca_chain(pki_mount)
        .await
        .context("Vault PKI ca_chain (refresh)")?
        .join("\n");

    let fullchain = bundle.fullchain_pem();
    let server_config = build_server_config(
        fullchain.as_bytes(),
        bundle.private_key.as_bytes(),
        Some(ca_chain_pem.as_bytes()),
        cfg.mtls_required,
    )?;
    config.reload_from_config(Arc::new(server_config));

    Ok(bundle)
}

fn compute_refresh_delay(ttl: Duration, refresh_pct: u8, jitter_pct: u8) -> Duration {
    let base_ms = ttl.as_millis() as u64 * u64::from(refresh_pct) / 100;
    let jitter_max_ms = base_ms * u64::from(jitter_pct) / 100;
    let jitter = pseudo_jitter_ms(jitter_max_ms);
    let final_ms = base_ms.saturating_add_signed(jitter);
    // Cota inferior 60 s para evitar bucle apretado si TTL emitido fue tiny.
    Duration::from_millis(final_ms.max(60_000))
}

fn pseudo_jitter_ms(max: u64) -> i64 {
    if max == 0 {
        return 0;
    }
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.subsec_nanos() as u64)
        .unwrap_or(0);
    (nanos % (2 * max + 1)) as i64 - max as i64
}

#[cfg(test)]
pub(crate) fn rcgen_test_chain() -> (String, String, String) {
    use rcgen::{CertificateParams, DistinguishedName, DnType, Issuer, KeyPair};

    let mut ca_params = CertificateParams::default();
    ca_params.distinguished_name = DistinguishedName::new();
    ca_params
        .distinguished_name
        .push(DnType::CommonName, "test-ca");
    ca_params.is_ca = rcgen::IsCa::Ca(rcgen::BasicConstraints::Unconstrained);
    let ca_key = KeyPair::generate().unwrap();
    let ca_issuer = Issuer::from_params(&ca_params, &ca_key);
    let ca = ca_params.self_signed(&ca_key).unwrap();

    let mut leaf_params = CertificateParams::new(vec!["localhost".into()]).unwrap();
    leaf_params.distinguished_name = DistinguishedName::new();
    leaf_params
        .distinguished_name
        .push(DnType::CommonName, "localhost");
    let leaf_key = KeyPair::generate().unwrap();
    let leaf = leaf_params.signed_by(&leaf_key, &ca_issuer).unwrap();

    (leaf.pem(), leaf_key.serialize_pem(), ca.pem())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_server_config_tls_simple_ok() {
        crate::init_crypto();
        let (leaf, key, _ca) = rcgen_test_chain();
        let _cfg = build_server_config(leaf.as_bytes(), key.as_bytes(), None, false)
            .expect("ServerConfig TLS sin client auth");
    }

    #[test]
    fn build_server_config_mtls_optional_ok() {
        crate::init_crypto();
        let (leaf, key, ca) = rcgen_test_chain();
        let cfg = build_server_config(leaf.as_bytes(), key.as_bytes(), Some(ca.as_bytes()), false)
            .expect("ServerConfig mTLS opcional");
        let _ = Arc::new(cfg);
    }

    #[test]
    fn build_server_config_mtls_required_ok() {
        crate::init_crypto();
        let (leaf, key, ca) = rcgen_test_chain();
        let cfg = build_server_config(leaf.as_bytes(), key.as_bytes(), Some(ca.as_bytes()), true)
            .expect("ServerConfig mTLS required");
        let _ = Arc::new(cfg);
    }

    #[test]
    fn parse_private_key_rechaza_pem_vacio() {
        let err = parse_private_key(b"").unwrap_err();
        let msg = format!("{err:#}");
        assert!(msg.contains("private key"), "msg fue: {msg}");
    }

    #[test]
    fn compute_refresh_delay_cota_inferior_60s() {
        let ttl = Duration::from_secs(10);
        let d = compute_refresh_delay(ttl, 50, 10);
        assert!(d.as_secs() >= 60);
    }

    #[test]
    fn compute_refresh_delay_normal_dentro_jitter() {
        let ttl = Duration::from_secs(7 * 24 * 60 * 60);
        let d = compute_refresh_delay(ttl, 50, 10);
        let base = 7 * 24 * 60 * 60 / 2;
        let jitter = base / 10;
        assert!(d.as_secs() >= base - jitter);
        assert!(d.as_secs() <= base + jitter);
    }
}
