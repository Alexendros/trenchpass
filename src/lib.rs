//! TrenchPass · librería compartida (re-exports y `AppState`).
//!
//! El binario en `main.rs` la consume. Los tests `cargo test` también.

pub mod audit;
pub mod auth;
pub mod config;
pub mod error;
pub mod otel;
pub mod security;
pub mod tools;
pub mod transport;
pub mod vault;

use std::sync::{Arc, OnceLock};

/// Instala el `CryptoProvider` `aws-lc-rs` como default proceso-global.
/// Idempotente vía `OnceLock`: la segunda llamada es no-op.
/// Si otro provider (e.g. `ring` instalado por una dep transitiva con ctor)
/// ya estaba registrado, `install_default` devuelve `Err` y emitimos un
/// `warn!` ruidoso — la garantía FIPS-friendly se pierde silenciosamente
/// si no se vigila este log.
/// DEBE invocarse antes de cualquier `ServerConfig::builder()` o
/// constructor de `rustls`.
pub fn init_crypto() {
    static ONCE: OnceLock<()> = OnceLock::new();
    ONCE.get_or_init(|| {
        if rustls::crypto::aws_lc_rs::default_provider()
            .install_default()
            .is_err()
        {
            tracing::warn!(
                target: "trenchpass.crypto",
                "aws-lc-rs no pudo instalarse como CryptoProvider default · otro provider \
                 (probablemente ring) ya estaba registrado · garantía FIPS-friendly perdida"
            );
        }
    });
}

use crate::audit::AuditStore;
use crate::config::Config;
use crate::security::{RateLimiter, ReplayCache};
use crate::tools::ToolRegistry;
use crate::vault::VaultClient;

/// Estado global compartido por handlers axum.
pub struct AppState {
    pub config: Config,
    pub vault: VaultClient,
    pub audit: AuditStore,
    pub tools: Arc<ToolRegistry>,
    pub rate_limiter: RateLimiter,
    pub replay_cache: ReplayCache,
}

impl AppState {
    pub async fn build(config: Config) -> error::Result<Arc<Self>> {
        let vault = VaultClient::new(&config.vault)?;
        let audit = AuditStore::connect(&config.database).await?;
        let tools = ToolRegistry::build();
        let rate_limiter = RateLimiter::default();
        let replay_cache = ReplayCache::new();
        Ok(Arc::new(Self {
            config,
            vault,
            audit,
            tools,
            rate_limiter,
            replay_cache,
        }))
    }
}
