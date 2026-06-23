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
    // El planning es síncrono: extrae del request todo lo necesario (token, CN)
    // ANTES de cualquier await. Así el future del middleware nunca retiene un
    // `&Request` (que no es `Sync`) cruzando el await del lookup en Vault — de lo
    // contrario el future deja de ser `Send` y axum rechaza el middleware.
    let consumer = match plan_auth(&state, &req)? {
        AuthPlan::Resolved(consumer) => consumer,
        AuthPlan::NeedsVault { consumer_id, token } => {
            bearer::resolve(&state.vault, &consumer_id, &token).await?
        }
    };

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

/// Resultado del planning síncrono de auth.
enum AuthPlan {
    /// Dev path · consumer ya resuelto (sin tocar Vault).
    Resolved(Consumer),
    /// Prod path · falta el lookup async en Vault con datos ya extraídos (owned).
    NeedsVault { consumer_id: String, token: String },
}

fn plan_auth(state: &AppState, req: &Request<Body>) -> Result<AuthPlan> {
    let token = bearer::extract(req.headers())?;

    // Dev sólo si NO es producción y hay dev_bearer configurado: resolver local
    // con scopes wildcard, mTLS opcional. `config.rs` ya prohíbe dev_bearer en
    // producción, así que esta rama es inalcanzable allí.
    if !state.config.env.is_production() {
        if let Some(expected) = state.config.dev_bearer.as_deref() {
            let consumer = bearer::resolve_dev(token, Some(expected))?;
            if let Some(cn) = mtls::extract_cn(
                req,
                state.config.tls.mtls_required,
                state.config.tls.mode,
                state.config.tls.mtls_header_trusted,
            )? {
                mtls::assert_match(&cn, &consumer.id)?;
            }
            return Ok(AuthPlan::Resolved(consumer));
        }
    }

    // Producción · multi-consumidor: el CN del cert mTLS da la identidad, y el
    // token sólo se compara (constant-time) contra el `token_hash` custodiado en
    // Vault. El orden se invierte respecto a dev: CN primero (índice), bearer
    // después (prueba). mTLS es obligatorio en este path.
    let cn = mtls::extract_cn(
        req,
        true,
        state.config.tls.mode,
        state.config.tls.mtls_header_trusted,
    )?
    .ok_or(Error::Auth(crate::error::AuthError::MissingClientCert))?;
    Ok(AuthPlan::NeedsVault {
        consumer_id: cn,
        token: token.to_string(),
    })
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
