//! Validación del cert mTLS — CN del cert == `consumer_id` del Bearer.
//!
//! Dos fuentes soportadas (en orden de preferencia):
//! 1. **Direct rustls path** (PR3.2+): cert validado por
//!    `WebPkiClientVerifier` durante el handshake e inyectado en
//!    `Request::extensions()` por
//!    [`crate::transport::peer_cert::PeerCertAcceptor`].
//! 2. **Traefik header path** (legacy): header `X-Forwarded-Tls-Client-Cert`
//!    con el cert en URI-encoded PEM tras Traefik terminar mTLS upstream.
//!
//! `extract_cn` toma una `Request` completa (no sólo `HeaderMap`) para
//! poder dispatchear entre ambas fuentes.

use axum::body::Body;
use axum::extract::Request;
use axum::http::HeaderMap;
use percent_encoding::percent_decode_str;
use rustls_pki_types::CertificateDer;
use tracing::warn;
use x509_parser::pem::Pem;
use x509_parser::prelude::FromDer;

use crate::config::TlsMode;
use crate::error::{AuthError, Error, Result};
use crate::transport::PeerCertificate;

/// Header convencional que Traefik añade tras verificar mTLS upstream.
const HDR_CLIENT_CERT: &str = "x-forwarded-tls-client-cert";

/// Devuelve el CN extraído del cert presentado por el cliente.
///
/// - Si la conexión TLS llegó directamente al gateway (rustls verifier),
///   parsea desde la `PeerCertificate` extension.
/// - Si no, intenta el header de Traefik, pero sólo si `tls_mode != Off` o
///   `header_trusted == true` (evita aceptar un header falsificable en modo
///   plain sin proxy validador).
/// - Si ninguna fuente está disponible y `required=false`, devuelve `Ok(None)`.
/// - Si `required=true` y ninguna fuente disponible, error.
pub fn extract_cn(
    req: &Request<Body>,
    required: bool,
    tls_mode: TlsMode,
    header_trusted: bool,
) -> Result<Option<String>> {
    // 1) Direct path · cert inyectado por PeerCertAcceptor
    if let Some(cert) = req.extensions().get::<PeerCertificate>() {
        return parse_cn_from_der(&cert.0).map(Some);
    }
    // 2) Traefik header path (sólo si el contexto lo hace creíble)
    extract_cn_from_headers(req.headers(), required, tls_mode, header_trusted)
}

/// Compatibilidad legacy · ruta header-only para llamadas que no tienen
/// `Request` completo a mano. `auth_middleware` debe preferir [`extract_cn`].
pub fn extract_cn_from_headers(
    headers: &HeaderMap,
    required: bool,
    tls_mode: TlsMode,
    header_trusted: bool,
) -> Result<Option<String>> {
    // Rechazar header si TLS está off y no se ha declarado explícitamente de confianza.
    if tls_mode == TlsMode::Off && !header_trusted {
        if required {
            return Err(Error::Auth(AuthError::MissingClientCert));
        }
        return Ok(None);
    }
    let Some(raw) = headers.get(HDR_CLIENT_CERT) else {
        if required {
            return Err(Error::Auth(AuthError::MissingClientCert));
        }
        return Ok(None);
    };
    let pem_uri = raw
        .to_str()
        .map_err(|_| Error::Auth(AuthError::MissingClientCert))?;

    parse_cn_from_pem_uri(pem_uri).map(Some)
}

fn parse_cn_from_pem_uri(pem_uri_encoded: &str) -> Result<String> {
    // RFC 3986 percent-decoding completo (cubre %09, %0A, %0D, etc.).
    let decoded = percent_decode_str(pem_uri_encoded)
        .decode_utf8()
        .map_err(|e| {
            warn!(target: "auth.mtls", "percent decode falló: {e}");
            Error::Auth(AuthError::MissingClientCert)
        })?;

    let mut reader = std::io::Cursor::new(decoded.as_bytes());
    let (pem, _) = Pem::read(&mut reader).map_err(|e| {
        warn!(target: "auth.mtls", "pem parse error: {e}");
        Error::Auth(AuthError::MissingClientCert)
    })?;
    let cert = pem.parse_x509().map_err(|e| {
        warn!(target: "auth.mtls", "x509 parse error: {e}");
        Error::Auth(AuthError::MissingClientCert)
    })?;

    cn_from_x509(&cert)
}

fn parse_cn_from_der(cert: &CertificateDer<'_>) -> Result<String> {
    let (_, parsed) =
        x509_parser::certificate::X509Certificate::from_der(cert.as_ref()).map_err(|e| {
            warn!(target: "auth.mtls", "x509 from_der error: {e}");
            Error::Auth(AuthError::MissingClientCert)
        })?;
    cn_from_x509(&parsed)
}

fn cn_from_x509(cert: &x509_parser::certificate::X509Certificate<'_>) -> Result<String> {
    cert.subject()
        .iter_common_name()
        .next()
        .and_then(|cn| cn.as_str().ok().map(|s| s.to_owned()))
        .ok_or_else(|| {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::TlsMode;
    use std::sync::Arc;

    fn rcgen_leaf() -> (Vec<u8>, String) {
        use rcgen::{CertificateParams, DistinguishedName, DnType, KeyPair};
        let mut params = CertificateParams::new(vec!["test.local".into()]).unwrap();
        params.distinguished_name = DistinguishedName::new();
        params
            .distinguished_name
            .push(DnType::CommonName, "consumer-A");
        let key = KeyPair::generate().unwrap();
        let cert = params.self_signed(&key).unwrap();
        (cert.der().to_vec(), "consumer-A".to_string())
    }

    #[test]
    fn parse_cn_from_der_extracts_cn() {
        let (der, expected) = rcgen_leaf();
        let cd = CertificateDer::from(der);
        let cn = parse_cn_from_der(&cd).expect("cn");
        assert_eq!(cn, expected);
    }

    #[test]
    fn extract_cn_prefers_extension_over_header() {
        let (der, expected) = rcgen_leaf();
        let cd = CertificateDer::from(der);
        let pc = PeerCertificate(Arc::new(cd));

        let mut req = Request::builder()
            .uri("/")
            .header(HDR_CLIENT_CERT, "garbage-not-a-pem")
            .body(Body::empty())
            .unwrap();
        req.extensions_mut().insert(pc);

        let cn = extract_cn(&req, true, TlsMode::Off, false)
            .expect("cn")
            .expect("some");
        assert_eq!(cn, expected);
    }

    #[test]
    fn extract_cn_falls_back_to_header_when_no_extension() {
        // PEM mínimo de un cert con CN=test
        // (usar rcgen para producir uno y URI-encoded percent encoding)
        let mut params = rcgen::CertificateParams::new(vec!["x.local".into()]).unwrap();
        params.distinguished_name = rcgen::DistinguishedName::new();
        params
            .distinguished_name
            .push(rcgen::DnType::CommonName, "header-CN");
        let key = rcgen::KeyPair::generate().unwrap();
        let cert = params.self_signed(&key).unwrap();
        let pem = cert.pem();

        let encoded: String =
            percent_encoding::utf8_percent_encode(&pem, percent_encoding::NON_ALPHANUMERIC)
                .to_string();

        let req = Request::builder()
            .uri("/")
            .header(HDR_CLIENT_CERT, encoded)
            .body(Body::empty())
            .unwrap();

        let cn = extract_cn(&req, true, TlsMode::Static, false)
            .expect("cn")
            .expect("some");
        assert_eq!(cn, "header-CN");
    }

    #[test]
    fn extract_cn_rejects_header_in_off_mode_when_untrusted() {
        let req = Request::builder()
            .uri("/")
            .header(HDR_CLIENT_CERT, "garbage-not-a-pem")
            .body(Body::empty())
            .unwrap();
        let err = extract_cn(&req, true, TlsMode::Off, false).unwrap_err();
        matches!(err, Error::Auth(AuthError::MissingClientCert));
    }

    #[test]
    fn extract_cn_missing_required_errors() {
        let req = Request::builder().uri("/").body(Body::empty()).unwrap();
        let err = extract_cn(&req, true, TlsMode::Off, false).unwrap_err();
        matches!(err, Error::Auth(AuthError::MissingClientCert));
    }

    #[test]
    fn extract_cn_missing_optional_returns_none() {
        let req = Request::builder().uri("/").body(Body::empty()).unwrap();
        assert!(extract_cn(&req, false, TlsMode::Off, false)
            .unwrap()
            .is_none());
    }

    #[test]
    fn assert_match_ok_and_mismatch() {
        assert!(assert_match("a", "a").is_ok());
        let err = assert_match("a", "b").unwrap_err();
        matches!(err, Error::Auth(AuthError::CnMismatch { .. }));
    }
}
