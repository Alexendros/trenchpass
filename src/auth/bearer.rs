//! Validación del token Bearer.
//!
//! Dos resolvers:
//! - [`resolve_dev`]: modo dev (`TRENCHPASS_DEV_BEARER`), scopes wildcard.
//! - [`resolve`]: producción · lee `secret/consumers/<id>` en Vault (campos
//!   `token_hash` = hex(sha256(token)), `scopes` = array, `ttl` opcional) y
//!   compara el hash en tiempo constante.

use axum::http::HeaderMap;
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq;

use super::Consumer;
use crate::error::{AuthError, Error, Result};
use crate::tools::shared::extract_string_field;
use crate::vault::VaultClient;

const HEADER: &str = "authorization";

/// Prefijo del path KV v2 donde viven los consumidores (`<mount>/consumers/<id>`).
const CONSUMERS_PREFIX: &str = "consumers";

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

/// Dev resolver: si `expected_dev` está set y matchea, devuelve consumer
/// "dev" con scopes wildcard. Sólo activo fuera de producción.
pub fn resolve_dev(token: &str, expected_dev: Option<&str>) -> Result<Consumer> {
    match expected_dev {
        Some(expected) if expected == token => Ok(Consumer::dev("dev-consumer")),
        _ => Err(Error::Auth(AuthError::InvalidBearer)),
    }
}

/// Resolver de producción · multi-consumidor.
///
/// `consumer_id` proviene del CN del cert mTLS (no del token), de modo que el
/// token jamás se usa como índice. Lee `<kv_mount>/consumers/<id>`:
/// - `token_hash`: hex de `sha256(token)`.
/// - `scopes`: array de strings (`"stripe:list_*"`, `"notion:*"`, `"*"`, `"admin:*"`).
/// - `ttl` (opcional): segundos de validez declarados (informativo; el TTL real
///   lo aplica el cache de Vault y la expiración del cert).
///
/// La comparación del hash es constant-time para no filtrar información por
/// timing. Un mismatch devuelve `InvalidBearer` (401), indistinguible de un
/// consumidor inexistente desde el punto de vista del atacante.
pub async fn resolve(vault: &VaultClient, consumer_id: &str, token: &str) -> Result<Consumer> {
    let path = format!("{CONSUMERS_PREFIX}/{consumer_id}");
    let secret = vault.secret(&path).await?;

    let stored_hash = extract_string_field(&secret.data, "token_hash", &path)?;
    let computed = hex_sha256(token);
    let matches: bool = computed.as_bytes().ct_eq(stored_hash.as_bytes()).into();
    if !matches {
        return Err(Error::Auth(AuthError::InvalidBearer));
    }

    let scopes = extract_scopes(&secret.data, &path)?;
    let ttl_secs = extract_ttl(&secret.data);

    Ok(Consumer {
        id: consumer_id.to_string(),
        scopes,
        ttl_secs,
    })
}

/// hex(sha256(token)) en minúsculas · formato canónico de `token_hash`.
fn hex_sha256(token: &str) -> String {
    let digest = Sha256::digest(token.as_bytes());
    let mut out = String::with_capacity(digest.len() * 2);
    for byte in digest {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

/// Extrae `scopes` como `Vec<String>` del secret (soporta envelope KV v2 `{data: …}`
/// y cuerpo plano). Un array vacío o ausente es error: un consumidor sin scopes
/// no puede invocar nada y casi siempre indica un secret mal provisionado.
fn extract_scopes(data: &serde_json::Value, path: &str) -> Result<Vec<String>> {
    let arr = data
        .get("data")
        .and_then(|d| d.get("scopes"))
        .or_else(|| data.get("scopes"))
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| Error::Vault(format!("campo `scopes` ausente o no-array en {path}")))?;

    let scopes: Vec<String> = arr
        .iter()
        .filter_map(|v| v.as_str().map(str::to_string))
        .collect();

    if scopes.is_empty() {
        return Err(Error::Vault(format!("`scopes` vacío en {path}")));
    }
    Ok(scopes)
}

/// `ttl` opcional (segundos). Ausente → `None`.
fn extract_ttl(data: &serde_json::Value) -> Option<u64> {
    data.get("data")
        .and_then(|d| d.get("ttl"))
        .or_else(|| data.get("ttl"))
        .and_then(serde_json::Value::as_u64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn vault_with(consumer_id: &str, body: serde_json::Value) -> VaultClient {
        VaultClient::test_with_secret(&format!("{CONSUMERS_PREFIX}/{consumer_id}"), body)
    }

    #[test]
    fn hex_sha256_conocido() {
        // sha256("") = e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855
        assert_eq!(
            hex_sha256(""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }

    #[tokio::test]
    async fn resolve_consumer_valido() {
        let token = "s3cr3t-token";
        let vault = vault_with(
            "stripe-fetcher",
            json!({"data": {
                "token_hash": hex_sha256(token),
                "scopes": ["stripe:list_subscriptions", "stripe:list_charges"],
                "ttl": 3600,
            }}),
        );
        let consumer = resolve(&vault, "stripe-fetcher", token).await.unwrap();
        assert_eq!(consumer.id, "stripe-fetcher");
        assert_eq!(consumer.ttl_secs, Some(3600));
        assert_eq!(consumer.scopes.len(), 2);
    }

    #[tokio::test]
    async fn resolve_token_mismatch_es_401() {
        let vault = vault_with(
            "notion-fetcher",
            json!({"data": {"token_hash": hex_sha256("right"), "scopes": ["notion:*"]}}),
        );
        let err = resolve(&vault, "notion-fetcher", "wrong")
            .await
            .unwrap_err();
        assert!(matches!(err, Error::Auth(AuthError::InvalidBearer)));
    }

    #[tokio::test]
    async fn resolve_scopes_vacio_es_error() {
        let vault = vault_with(
            "broken",
            json!({"data": {"token_hash": hex_sha256("t"), "scopes": []}}),
        );
        let err = resolve(&vault, "broken", "t").await.unwrap_err();
        assert!(matches!(err, Error::Vault(_)));
    }

    #[tokio::test]
    async fn resolve_respeta_scope_acotado() {
        // Un fetcher de Stripe NO debe poder invocar tools de Notion.
        let token = "tk";
        let vault = vault_with(
            "stripe-fetcher",
            json!({"data": {"token_hash": hex_sha256(token), "scopes": ["stripe:*"]}}),
        );
        let consumer = resolve(&vault, "stripe-fetcher", token).await.unwrap();
        crate::auth::scope::check("stripe.list_subscriptions", &consumer.scopes).unwrap();
        let denied = crate::auth::scope::check("notion.search", &consumer.scopes).unwrap_err();
        assert!(matches!(denied, Error::ScopeViolation { .. }));
    }
}
