//! Smoke directo de `sync::detect_drift` contra Vault dev.
//!
//! Verifica los 4 casos del test plan PR4 sin necesidad de Postgres:
//!   1. Limpio: manifest declara `x/y` con expected_keys=[token]; vault tiene
//!      `data.token=abc` → 0 drifts.
//!   2. Missing: manifest declara `a/b`; vault sin esa entrada → 1 MissingFromVault.
//!   3. Extra: vault tiene `extra/path`; manifest no lo declara → 1 ExtraInVault.
//!   4. KeyMismatch: manifest expected_keys=[token]; vault data tiene
//!      {api_key:x} → 1 KeyMismatch.
//!
//! Pre-requisitos:
//!   `vault server -dev -dev-root-token-id=root` en :8200.
//! Uso:
//!   `VAULT_ADDR=http://127.0.0.1:8200 VAULT_TOKEN=root cargo run --example drift_smoke`

use std::time::Duration;

use anyhow::{Context, Result};
use trenchpass::config::VaultConfig;
use trenchpass::sync::{detect_drift, ExpectedSecret, Manifest, ManifestSource};
use trenchpass::vault::VaultClient;
use vaultrs::kv2;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "trenchpass=debug,info".parse().unwrap()),
        )
        .with_target(true)
        .init();

    let addr = std::env::var("VAULT_ADDR").unwrap_or_else(|_| "http://127.0.0.1:8200".into());
    let token = std::env::var("VAULT_TOKEN").unwrap_or_else(|_| "root".into());
    let mount = "secret";

    let cfg = VaultConfig {
        addr,
        token,
        kv_mount: mount.into(),
        pki_mount: "pki".into(),
        cache_ttl: Duration::from_secs(0), // sin cache para tests
    };
    let vault = VaultClient::new(&cfg).context("VaultClient::new")?;

    // === Setup KV state ===
    println!("[setup] poblando vault: x/y · extra/path · z/keymismatch");
    let raw = vault_raw(&cfg)?;
    kv2::set(
        &raw,
        mount,
        "x/y",
        &serde_json::json!({"token": "abc"}),
    )
    .await?;
    kv2::set(
        &raw,
        mount,
        "extra/path",
        &serde_json::json!({"x": 1}),
    )
    .await?;
    kv2::set(
        &raw,
        mount,
        "z/keymismatch",
        &serde_json::json!({"api_key": "wrong-key"}),
    )
    .await?;

    // === Manifest ===
    let manifest = Manifest {
        version: 1,
        vault_id: "smoke".into(),
        secrets: vec![
            ExpectedSecret {
                path: "x/y".into(),
                expected_keys: vec!["token".into()],
            },
            ExpectedSecret {
                path: "a/b".into(), // Missing
                expected_keys: vec!["secret".into()],
            },
            ExpectedSecret {
                path: "z/keymismatch".into(),
                expected_keys: vec!["token".into()],
            },
        ],
    };
    let source = ManifestSource::new(manifest);

    // === Run drift ===
    println!("[smoke] ejecutando detect_drift");
    let events = detect_drift(&source, &vault, mount).await?;
    println!("[smoke] {} drift event(s):", events.len());
    for ev in &events {
        let kind = ev.kind();
        let path = ev.path();
        let detail = serde_json::to_string(&ev).unwrap_or_default();
        println!("  - kind={kind:<22} path={path:<20} detail={detail}");
    }

    // === Asserts ===
    let kinds: Vec<&str> = events.iter().map(|e| e.kind()).collect();
    let paths: Vec<&str> = events.iter().map(|e| e.path()).collect();
    assert!(
        kinds.contains(&"missing_from_vault"),
        "falta MissingFromVault"
    );
    assert!(paths.contains(&"a/b"), "Missing debe ser sobre a/b");
    assert!(kinds.contains(&"extra_in_vault"), "falta ExtraInVault");
    assert!(paths.contains(&"extra/path"), "Extra debe ser sobre extra/path");
    assert!(kinds.contains(&"key_mismatch"), "falta KeyMismatch");
    assert!(
        paths.contains(&"z/keymismatch"),
        "KeyMismatch debe ser sobre z/keymismatch"
    );
    // x/y debe NO aparecer (caso limpio)
    assert!(
        !paths.contains(&"x/y"),
        "x/y no debería aparecer (caso limpio)"
    );

    println!("[smoke] OK · 4 casos verificados (1 limpio + 3 drift)");
    Ok(())
}

fn vault_raw(cfg: &VaultConfig) -> Result<vaultrs::client::VaultClient> {
    use vaultrs::client::VaultClientSettingsBuilder;
    let settings = VaultClientSettingsBuilder::default()
        .address(cfg.addr.clone())
        .token(cfg.token.clone())
        .build()?;
    Ok(vaultrs::client::VaultClient::new(settings)?)
}
