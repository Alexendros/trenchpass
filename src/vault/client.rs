//! Wrapper sobre `vaultrs` con cache `DashMap`.
//!
//! Estrategia:
//! - `secret(path)` → primero cache; si miss o expirado, fetch + insert.
//! - Versión del KV v2 ignorada en PR1 (siempre `latest`); PR4 añade pin por versión.
//! - Reaching Vault está envuelto en spans `tracing` para que SigNoz vea la latencia.

use std::sync::Arc;
use std::time::{Duration, Instant};

use dashmap::DashMap;
use tracing::instrument;
use vaultrs::client::{VaultClient as RawVaultClient, VaultClientSettingsBuilder};
use vaultrs::kv2;

use crate::config::VaultConfig;
use crate::error::{Error, Result};

#[derive(Clone, Debug)]
pub struct Secret {
    pub data: serde_json::Value,
    pub fetched_at: Instant,
}

#[derive(Clone)]
pub struct VaultClient {
    raw: Arc<RawVaultClient>,
    cache: Arc<DashMap<String, Secret>>,
    kv_mount: Arc<str>,
    ttl: Duration,
}

impl VaultClient {
    pub fn new(cfg: &VaultConfig) -> Result<Self> {
        let settings = VaultClientSettingsBuilder::default()
            .address(cfg.addr.clone())
            .token(cfg.token.clone())
            .build()
            .map_err(|e| Error::Vault(format!("settings: {e}")))?;
        let raw =
            RawVaultClient::new(settings).map_err(|e| Error::Vault(format!("client: {e}")))?;

        Ok(Self {
            raw: Arc::new(raw),
            cache: Arc::new(DashMap::new()),
            kv_mount: Arc::from(cfg.kv_mount.as_str()),
            ttl: cfg.cache_ttl,
        })
    }

    /// Resuelve un secreto KV v2 (mount=`kv_mount`, path relativo).
    #[instrument(skip(self), fields(mount = %self.kv_mount))]
    pub async fn secret(&self, path: &str) -> Result<Secret> {
        if let Some(hit) = self.cache.get(path) {
            if hit.fetched_at.elapsed() < self.ttl {
                tracing::debug!(target: "vault.cache", "hit");
                return Ok(hit.clone());
            }
        }

        tracing::debug!(target: "vault.cache", "miss · fetching");
        let raw_value: serde_json::Value = kv2::read(self.raw.as_ref(), &self.kv_mount, path)
            .await
            .map_err(|e| Error::Vault(format!("read {path}: {e}")))?;

        let secret = Secret {
            data: raw_value,
            fetched_at: Instant::now(),
        };
        self.cache.insert(path.to_string(), secret.clone());
        Ok(secret)
    }

    /// Invalida una entrada del cache (uso: tras `vault kv put`).
    pub fn invalidate(&self, path: &str) {
        self.cache.remove(path);
    }

    /// Vacía el cache completo (uso: rotación masiva o vía-fax `rotate-all`).
    pub fn invalidate_all(&self) {
        self.cache.clear();
    }

    /// Constructor de tests · no toca Vault. Pre-llena el cache con `(path, value)`
    /// con TTL infinito efectivo. El raw client apunta a una dirección inexistente
    /// (nunca se invoca porque el cache hit corta antes).
    #[cfg(test)]
    pub fn test_with_secret(path: &str, value: serde_json::Value) -> Self {
        let settings = VaultClientSettingsBuilder::default()
            .address("http://127.0.0.1:1")
            .token("test")
            .build()
            .expect("test vault settings");
        let raw = RawVaultClient::new(settings).expect("test vault client");
        let cache = DashMap::new();
        cache.insert(
            path.to_string(),
            Secret {
                data: value,
                fetched_at: Instant::now(),
            },
        );
        Self {
            raw: Arc::new(raw),
            cache: Arc::new(cache),
            kv_mount: Arc::from("secret"),
            ttl: Duration::from_secs(3600),
        }
    }
}
