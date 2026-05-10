//! Tipos de error compartidos del gateway.

use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("config error: {0}")]
    Config(String),

    #[error("vault error: {0}")]
    Vault(String),

    #[error("audit store error: {0}")]
    Audit(#[from] sqlx::Error),

    #[error("upstream provider error: {0}")]
    Upstream(String),

    #[error("auth error: {0}")]
    Auth(AuthError),

    #[error("rate limit exceeded")]
    RateLimited,

    #[error("replay detected")]
    Replay,

    #[error("scope violation: required {required}, granted {granted:?}")]
    ScopeViolation {
        required: String,
        granted: Vec<String>,
    },

    #[error("not found: {0}")]
    NotFound(String),

    #[error("internal: {0}")]
    Internal(#[from] anyhow::Error),
}

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("missing bearer")]
    MissingBearer,
    #[error("invalid bearer")]
    InvalidBearer,
    #[error("missing client cert")]
    MissingClientCert,
    #[error("client cert revoked")]
    CertRevoked,
    #[error("cn mismatch: cert={cert}, bearer={bearer}")]
    CnMismatch { cert: String, bearer: String },
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let (status, code) = match &self {
            Error::Auth(AuthError::MissingBearer | AuthError::InvalidBearer) => {
                (StatusCode::UNAUTHORIZED, "unauthorized")
            }
            Error::Auth(AuthError::MissingClientCert) => {
                (StatusCode::UNAUTHORIZED, "missing_client_cert")
            }
            Error::Auth(AuthError::CertRevoked) => (StatusCode::UNAUTHORIZED, "cert_revoked"),
            Error::Auth(AuthError::CnMismatch { .. }) => (StatusCode::FORBIDDEN, "cn_mismatch"),
            Error::ScopeViolation { .. } => (StatusCode::FORBIDDEN, "scope_violation"),
            Error::RateLimited => (StatusCode::TOO_MANY_REQUESTS, "rate_limited"),
            Error::Replay => (StatusCode::CONFLICT, "replay_detected"),
            Error::NotFound(_) => (StatusCode::NOT_FOUND, "not_found"),
            Error::Vault(_) | Error::Upstream(_) => (StatusCode::BAD_GATEWAY, "upstream_error"),
            Error::Audit(_) | Error::Internal(_) | Error::Config(_) => {
                (StatusCode::INTERNAL_SERVER_ERROR, "internal_error")
            }
        };
        let body = json!({ "error": code, "message": self.to_string() });
        (status, axum::Json(body)).into_response()
    }
}

pub type Result<T, E = Error> = std::result::Result<T, E>;
