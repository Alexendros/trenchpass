//! Cliente Vault con cache en memoria (TTL configurable, default 60s).
//!
//! Hot path del gateway: cada `tools/*.call()` resuelve secretos vía
//! [`VaultClient::secret`]. El cache evita martillear Vault en ráfagas.

pub mod client;

pub use client::{Secret, VaultClient};
