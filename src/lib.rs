//! TrenchPass · librería compartida (re-exports y `AppState`).
//!
//! El binario en `main.rs` la consume. Los tests `cargo test` también.

pub mod audit;
pub mod auth;
pub mod config;
pub mod error;
pub mod fax;
pub mod otel;
pub mod security;
pub mod sync;
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

/// Estado global compartido por handlers axum y workers (sync, vía-fax, …).
pub struct AppState {
    pub config: Config,
    pub vault: VaultClient,
    pub audit: AuditStore,
    pub tools: Arc<ToolRegistry>,
    pub rate_limiter: RateLimiter,
    pub replay_cache: ReplayCache,
    /// Cert público del operador para verificar firmas vía-fax. `None` si
    /// `FAX_PGP_OPERATOR_CERT_PATH` no apunta a un PEM/armored válido (el
    /// worker rechazará todo mensaje con `FaxError::NoOperatorCert`).
    pub fax_operator_cert: Option<Arc<sequoia_openpgp::Cert>>,
}

impl AppState {
    pub async fn build(config: Config) -> error::Result<Arc<Self>> {
        let vault = VaultClient::new(&config.vault)?;
        let audit = AuditStore::connect(&config.database).await?;
        let tools = ToolRegistry::build();
        let rate_limiter = RateLimiter::default();
        let replay_cache = ReplayCache::new();
        let fax_operator_cert = load_operator_cert(&config.fax.operator_cert_path);
        Ok(Arc::new(Self {
            config,
            vault,
            audit,
            tools,
            rate_limiter,
            replay_cache,
            fax_operator_cert,
        }))
    }
}

fn load_operator_cert(path: &Option<std::path::PathBuf>) -> Option<Arc<sequoia_openpgp::Cert>> {
    use sequoia_openpgp::parse::Parse;
    let path = path.as_ref()?;
    match sequoia_openpgp::Cert::from_file(path) {
        Ok(cert) => {
            tracing::info!(
                target: "fax.boot",
                path = %path.display(),
                fpr = %cert.fingerprint(),
                "operator cert cargado"
            );
            Some(Arc::new(cert))
        }
        Err(e) => {
            tracing::error!(
                target: "fax.boot",
                path = %path.display(),
                error = %e,
                "no se pudo cargar operator cert · vía-fax rechazará todo mensaje"
            );
            None
        }
    }
}
