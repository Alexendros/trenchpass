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

use std::sync::Arc;

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
