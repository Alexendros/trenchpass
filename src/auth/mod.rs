//! Auth doble factor: Bearer (con scopes) + cert mTLS (CN = consumer ID).
//!
//! Flujo PR1:
//! 1. `bearer::extract` lee `Authorization: Bearer <token>`.
//! 2. `bearer::resolve` mira en Vault `secret/consumers/<id>` y obtiene scopes.
//! 3. `mtls::verify_cn` (PR3) compara `consumer_id` con CN del cert.
//! 4. `scope::check` valida que la tool requerida cabe en los scopes.

pub mod bearer;
pub mod mtls;
pub mod scope;

use serde::{Deserialize, Serialize};

/// Identidad efectiva del consumidor tras pasar las dos validaciones.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Consumer {
    pub id: String,
    pub scopes: Vec<String>,
    pub ttl_secs: Option<u64>,
}

impl Consumer {
    pub fn dev(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            scopes: vec!["*".into()],
            ttl_secs: None,
        }
    }
}
