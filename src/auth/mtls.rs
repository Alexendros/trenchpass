//! Validación del cert mTLS — CN del cert == `consumer_id` del Bearer.
//!
//! PR1: stub funcional (extrae CN si el header existe, devuelve `None` si no
//! y `required=false`). Activación real en PR3 cuando Vault PKI emita certs
//! por consumidor y Traefik pase el cert validado en el header
//! `X-Forwarded-Tls-Client-Cert` (formato URI-encoded PEM).

use axum::http::HeaderMap;
use tracing::warn;
use x509_parser::pem::Pem;

use crate::error::{AuthError, Error, Result};

/// Header convencional que Traefik añade tras verificar mTLS upstream.
const HDR_CLIENT_CERT: &str = "x-forwarded-tls-client-cert";

/// Devuelve el CN extraído del cert presentado por el cliente.
/// PR1: si el header no existe y `required=false`, devuelve `Ok(None)`.
pub fn extract_cn(headers: &HeaderMap, required: bool) -> Result<Option<String>> {
    let Some(raw) = headers.get(HDR_CLIENT_CERT) else {
        if required {
            return Err(Error::Auth(AuthError::MissingClientCert));
        }
        return Ok(None);
    };
    let pem_uri = raw
        .to_str()
        .map_err(|_| Error::Auth(AuthError::MissingClientCert))?;

    parse_cn_from_pem(pem_uri).map(Some)
}

fn parse_cn_from_pem(pem_uri_encoded: &str) -> Result<String> {
    // Decodificación naive del URI-encoding que aplica Traefik. PR3 sustituye
    // por `percent_encoding::percent_decode_str` para cobertura total.
    let pem_string = pem_uri_encoded
        .replace("%20", "\n")
        .replace("%2B", "+")
        .replace("%2F", "/")
        .replace("%3D", "=");

    let mut reader = std::io::Cursor::new(pem_string.as_bytes());
    let (pem, _) = Pem::read(&mut reader).map_err(|e| {
        warn!(target: "auth.mtls", "pem parse error: {e}");
        Error::Auth(AuthError::MissingClientCert)
    })?;
    let cert = pem.parse_x509().map_err(|e| {
        warn!(target: "auth.mtls", "x509 parse error: {e}");
        Error::Auth(AuthError::MissingClientCert)
    })?;

    // El iterador de `iter_common_name()` toma prestado de `cert`, que toma de `pem`.
    // Materializamos el CN a `String` propio antes de devolver para no propagar el lifetime.
    let cn_owned: Option<String> = cert
        .subject()
        .iter_common_name()
        .next()
        .and_then(|cn| cn.as_str().ok().map(|s| s.to_owned()));

    cn_owned.ok_or_else(|| {
        warn!(target: "auth.mtls", "cert sin CN");
        Error::Auth(AuthError::MissingClientCert)
    })
}

pub fn assert_match(cert_cn: &str, bearer_consumer: &str) -> Result<()> {
    if cert_cn != bearer_consumer {
        return Err(Error::Auth(AuthError::CnMismatch {
            cert: cert_cn.to_string(),
            bearer: bearer_consumer.to_string(),
        }));
    }
    Ok(())
}
