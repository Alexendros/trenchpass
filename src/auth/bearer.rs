//! Validación del token Bearer.
//!
//! En PR1 sólo soportamos modo dev (`TRENCHPASS_DEV_BEARER`). PR2 lee scopes
//! desde Vault `secret/consumers/<id>` (campos `token_hash`, `scopes`, `ttl`).

use axum::http::HeaderMap;

use super::Consumer;
use crate::error::{AuthError, Error, Result};

const HEADER: &str = "authorization";

pub fn extract(headers: &HeaderMap) -> Result<&str> {
    let raw = headers
        .get(HEADER)
        .ok_or(Error::Auth(AuthError::MissingBearer))?
        .to_str()
        .map_err(|_| Error::Auth(AuthError::InvalidBearer))?;
    let token = raw
        .strip_prefix("Bearer ")
        .or_else(|| raw.strip_prefix("bearer "))
        .ok_or(Error::Auth(AuthError::InvalidBearer))?;
    if token.is_empty() {
        return Err(Error::Auth(AuthError::InvalidBearer));
    }
    Ok(token)
}

/// PR1 dev resolver: si `expected_dev` está set y matchea, devuelve consumer
/// "dev" con scopes wildcard. PR2 reemplaza con lookup en Vault.
pub fn resolve_dev(token: &str, expected_dev: Option<&str>) -> Result<Consumer> {
    match expected_dev {
        Some(expected) if expected == token => Ok(Consumer::dev("dev-consumer")),
        _ => Err(Error::Auth(AuthError::InvalidBearer)),
    }
}
