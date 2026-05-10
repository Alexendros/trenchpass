//! Smoke runtime de `spawn_refresh_loop` contra Vault dev.
//!
//! Este ejemplo conecta a un Vault local en modo dev (token=`root` por defecto),
//! emite un leaf cert via PKI con TTL corto, y deja el refresh loop corriendo
//! para que el operador pueda observar:
//!
//! 1. Logs `cert reloaded` cuando funciona normal.
//! 2. Logs `refresh falló · reintentando con backoff corto · backoff_secs=30`
//!    cada 30 s cuando Vault está caído.
//! 3. Logs `cert reloaded` con un nuevo `serial` y verifier renovado tras
//!    `vault write pki_int/root/rotate`.
//!
//! Pre-requisitos (script en `examples/refresh_smoke_setup.sh`):
//! - `vault server -dev -dev-root-token-id=root` corriendo en :8200
//! - PKI mount habilitado, root CA generado, role `mcp-gateway` configurado.
//!
//! Uso:
//! ```bash
//! VAULT_ADDR=http://127.0.0.1:8200 VAULT_TOKEN=root \
//!   cargo run --example refresh_smoke
//! ```

use std::time::Duration;

use anyhow::{Context, Result};
use trenchpass::config::{TlsConfig, TlsMode};
use trenchpass::transport::mtls;
use trenchpass::vault::VaultClient;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "trenchpass=debug,info".parse().unwrap()),
        )
        .with_target(true)
        .init();

    trenchpass::init_crypto();

    let vault_addr = std::env::var("VAULT_ADDR").unwrap_or_else(|_| "http://127.0.0.1:8200".into());
    let vault_token = std::env::var("VAULT_TOKEN").unwrap_or_else(|_| "root".into());
    let pki_mount = std::env::var("VAULT_PKI_MOUNT").unwrap_or_else(|_| "pki_int".into());

    let vault_cfg = trenchpass::config::VaultConfig {
        addr: vault_addr,
        token: vault_token,
        kv_mount: "secret".into(),
        pki_mount: pki_mount.clone(),
        cache_ttl: Duration::from_secs(60),
    };
    let vault = VaultClient::new(&vault_cfg).context("VaultClient::new")?;

    // TTL muy corto (90 s) para que el primer refresh ocurra rápido.
    let tls_cfg = TlsConfig {
        mode: TlsMode::VaultPki,
        cert: None,
        key: None,
        client_ca: None,
        mtls_required: true,
        pki_role: "mcp-gateway".into(),
        pki_common_name: "trenchpass.local".into(),
        pki_alt_names: vec![],
        pki_cert_ttl: Duration::from_secs(90),
        pki_refresh_percent: 50,
        pki_refresh_jitter_percent: 10,
    };

    println!("[smoke] solicitando bootstrap cert · TTL=90s · refresh@45s±10%");
    let handle = mtls::build(&tls_cfg, Some(&vault), Some(&pki_mount))
        .await
        .context("mtls::build")?;
    let initial = handle
        .initial_bundle
        .clone()
        .expect("VaultPki produce initial_bundle");
    println!(
        "[smoke] bootstrap OK · serial={} ttl_real={}s",
        initial.serial_number,
        initial.ttl.as_secs()
    );

    let _refresh_handle = mtls::spawn_refresh_loop(
        handle.config.clone(),
        vault.clone(),
        pki_mount,
        tls_cfg,
        initial,
    );

    // Mantén el proceso vivo · el operador hace los gestos (kill vault, rotate).
    let dur_str = std::env::var("SMOKE_RUN_SECS").unwrap_or_else(|_| "180".into());
    let dur: u64 = dur_str.parse().unwrap_or(180);
    println!("[smoke] loop activo durante {dur}s · operador puede:");
    println!("        - parar vault para verificar backoff 30s");
    println!("        - rotar root CA: vault write -force pki_int/root/rotate/internal common_name=test ttl=87600h");
    tokio::time::sleep(Duration::from_secs(dur)).await;
    println!("[smoke] FIN");
    Ok(())
}
