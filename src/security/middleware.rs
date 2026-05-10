//! Middleware único que ata: extract bearer → CN match → rate limit → replay → continúa.
//!
//! En PR1 sólo aplicamos bearer (modo dev) y dejamos hooks marcados para PR3+.
//! La validación de scopes se ejecuta dentro del tool router (`tools::dispatch`)
//! porque depende del nombre de la tool invocada.

use std::sync::Arc;
use std::time::SystemTime;

use axum::{
    body::Body,
    extract::{Request, State},
    http::HeaderMap,
    middleware::Next,
    response::Response,
};

use crate::auth::{bearer, mtls, Consumer};
use crate::error::{Error, Result};
use crate::AppState;

/// Header convencional añadido por consumidores; PR3 lo refuerza.
const HDR_NONCE: &str = "x-trenchpass-nonce";
const HDR_TIMESTAMP: &str = "x-trenchpass-timestamp";

pub async fn auth_middleware(
    State(state): State<Arc<AppState>>,
    mut req: Request<Body>,
    next: Next,
) -> Result<Response> {
    let consumer = authorize(&state, req.headers())?;

    if !state.rate_limiter.check(&consumer.id) {
        return Err(Error::RateLimited);
    }

    if let Some((nonce, ts)) = extract_replay_headers(req.headers()) {
        let now = unix_now();
        if !state.replay_cache.check(&nonce, ts, now) {
            return Err(Error::Replay);
        }
    }

    // Inyecta el consumer en extensions para que los handlers lo lean.
    req.extensions_mut().insert(consumer);
    Ok(next.run(req).await)
}

fn authorize(state: &AppState, headers: &HeaderMap) -> Result<Consumer> {
    let token = bearer::extract(headers)?;

    // PR1: solo dev resolver. PR2 sustituye por lookup en Vault.
    let consumer = bearer::resolve_dev(token, state.config.dev_bearer.as_deref())?;

    // PR3 activa mTLS estricto:
    if let Some(cn) = mtls::extract_cn(headers, state.config.tls.mtls_required)? {
        mtls::assert_match(&cn, &consumer.id)?;
    }
    Ok(consumer)
}

fn extract_replay_headers(headers: &HeaderMap) -> Option<(String, i64)> {
    let nonce = headers.get(HDR_NONCE)?.to_str().ok()?.to_string();
    let ts = headers.get(HDR_TIMESTAMP)?.to_str().ok()?.parse().ok()?;
    Some((nonce, ts))
}

fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or_default()
}
