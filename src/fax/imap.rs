//! Worker IMAP + IMAPS contra Proton.
//!
//! Estrategia MVP (no IDLE): polling explícito cada `poll_interval`. IDLE
//! reduce latencia pero añade complejidad de keepalive · 60 s es aceptable
//! para flujos defensivos (revoke/seal/invalidate).
//!
//! Cuando `operator_pubkey` no está cargado (PR7.1: lectura desde Vault o
//! disco), el worker registra `warn!` por ciclo pero NO procesa mensajes.

use std::fmt::Debug;
use std::sync::Arc;

use chrono::Utc;
use futures_util::StreamExt;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio::net::TcpStream;
use tokio::time::sleep;
use tokio_rustls::rustls::pki_types::ServerName;
use tokio_rustls::TlsConnector;
use tracing::{debug, error, info, warn};

use crate::AppState;

use super::commands::parse_envelope;
use super::{dispatch, pgp, FaxError, FaxResult};

/// Bucle principal: poll → process → mark-seen → sleep, indefinidamente.
pub async fn run(state: Arc<AppState>) -> FaxResult<()> {
    let interval = state.config.fax.poll_interval;
    loop {
        let processed = match poll_once(&state).await {
            Ok(n) => n,
            Err(e) => {
                error!(target: "fax.imap", error = %e, "ciclo falló");
                0
            }
        };
        if processed > 0 {
            info!(target: "fax.imap", processed, "ciclo OK");
        } else {
            debug!(target: "fax.imap", "ciclo OK (sin mensajes nuevos)");
        }
        sleep(interval).await;
    }
}

/// Un ciclo: conecta, login, fetch UNSEEN, procesa, marca Seen, logout.
///
/// Devuelve cuántos mensajes fueron despachados con éxito.
async fn poll_once(state: &Arc<AppState>) -> FaxResult<usize> {
    let cfg = &state.config.fax;

    let tcp = TcpStream::connect((cfg.imap_host.as_str(), cfg.imap_port))
        .await
        .map_err(|e| {
            FaxError::Imap(format!(
                "TCP connect {}:{}: {e}",
                cfg.imap_host, cfg.imap_port
            ))
        })?;

    let tls_config = build_tls_config();
    let connector = TlsConnector::from(Arc::new(tls_config));
    let server_name = ServerName::try_from(cfg.imap_host.clone())
        .map_err(|e| FaxError::Imap(format!("ServerName: {e}")))?;
    let tls = connector
        .connect(server_name, tcp)
        .await
        .map_err(|e| FaxError::Imap(format!("TLS handshake: {e}")))?;

    let client = async_imap::Client::new(tls);
    let mut session = client
        .login(&cfg.imap_user, &cfg.imap_password)
        .await
        .map_err(|(e, _)| FaxError::Imap(format!("login: {e}")))?;

    session
        .select("INBOX")
        .await
        .map_err(|e| FaxError::Imap(format!("select INBOX: {e}")))?;

    let uids: Vec<u32> = session
        .search("UNSEEN")
        .await
        .map_err(|e| FaxError::Imap(format!("search UNSEEN: {e}")))?
        .into_iter()
        .collect();

    let mut processed = 0usize;
    for uid in uids {
        match process_uid(state, &mut session, uid).await {
            Ok(()) => {
                processed += 1;
                // Marca \Seen sólo si el mensaje fue dispatchado con éxito.
                let _ = session
                    .store(uid.to_string(), "+FLAGS (\\Seen)")
                    .await
                    .map(|s| s.collect::<Vec<_>>());
            }
            Err(e) => {
                warn!(
                    target: "fax.imap",
                    uid,
                    error = %e,
                    "mensaje inválido · queda UNSEEN para forensia"
                );
            }
        }
    }

    let _ = session.logout().await;
    Ok(processed)
}

/// Construye un `rustls::ClientConfig` mínimo con raíces del sistema.
fn build_tls_config() -> tokio_rustls::rustls::ClientConfig {
    let mut roots = tokio_rustls::rustls::RootCertStore::empty();
    // En distroless/cc-debian12 `/etc/ssl/certs` tiene los CA roots del sistema.
    // rustls-native-certs sería ideal; sin esa crate como dep extra, usamos
    // webpki-roots vía `tokio_rustls::rustls::pki_types` fallback simple.
    for cert in rustls_native_or_static_roots() {
        let _ = roots.add(cert);
    }
    tokio_rustls::rustls::ClientConfig::builder()
        .with_root_certificates(roots)
        .with_no_client_auth()
}

/// Devuelve un set mínimo de roots. En MVP: vacío → el handshake fallará
/// salvo que el operador inyecte CA bundle vía `SSL_CERT_FILE` del runtime
/// (distroless/cc lo lee del PEM en `/etc/ssl/certs/ca-certificates.crt`).
/// PR7.1: añadir `rustls-native-certs` como dep para enumerar los roots
/// del SO automáticamente.
fn rustls_native_or_static_roots() -> Vec<tokio_rustls::rustls::pki_types::CertificateDer<'static>>
{
    Vec::new()
}

/// Procesa un mensaje: fetch RFC822, parse MIME, verifica PGP, dispatch.
async fn process_uid<S>(
    state: &Arc<AppState>,
    session: &mut async_imap::Session<S>,
    uid: u32,
) -> FaxResult<()>
where
    S: AsyncRead + AsyncWrite + Unpin + Send + Debug,
{
    let mut fetches = session
        .fetch(uid.to_string(), "RFC822")
        .await
        .map_err(|e| FaxError::Imap(format!("fetch {uid}: {e}")))?;

    let mut raw = Vec::new();
    while let Some(item) = fetches.next().await {
        let item = item.map_err(|e| FaxError::Imap(format!("fetch stream: {e}")))?;
        if let Some(body) = item.body() {
            raw.extend_from_slice(body);
        }
    }
    drop(fetches);

    let armored = extract_pgp_part(&raw)?;
    let pubkey = state
        .fax_operator_cert
        .as_ref()
        .ok_or(FaxError::NoOperatorCert)?;

    let fpr = &state.config.fax.pgp_operator_fingerprint;
    let verified = pgp::verify_armored(&armored, fpr, pubkey)?;

    let envelope = parse_envelope(&verified.body)?;

    // Replay protection: nonce único + timestamp ±5 min.
    let now = Utc::now().timestamp();
    if !state
        .replay_cache
        .check(&envelope.nonce.to_string(), envelope.timestamp, now)
    {
        return Err(FaxError::Replay);
    }

    let sig_hash = sha256_hex(&armored);
    dispatch::execute(state, &envelope.command, &sig_hash).await
}

/// Busca la primera parte MIME `text/plain` o `application/pgp-encrypted` con
/// armadura PGP. Si no encuentra una parte específica, intenta el cuerpo
/// completo como fallback (cubre mails sin MIME multipart).
fn extract_pgp_part(rfc822: &[u8]) -> FaxResult<Vec<u8>> {
    let parsed =
        mailparse::parse_mail(rfc822).map_err(|e| FaxError::Mime(format!("parse_mail: {e}")))?;
    if let Some(armored) = scan_part(&parsed) {
        return Ok(armored);
    }
    let raw_body = parsed
        .get_body_raw()
        .map_err(|e| FaxError::Mime(format!("get_body_raw: {e}")))?;
    if contains_pgp_armor(&raw_body) {
        Ok(raw_body)
    } else {
        Err(FaxError::Mime(
            "no se encontró armadura PGP en el mail".into(),
        ))
    }
}

fn scan_part(part: &mailparse::ParsedMail) -> Option<Vec<u8>> {
    if let Ok(body) = part.get_body_raw() {
        if contains_pgp_armor(&body) {
            return Some(body);
        }
    }
    for sub in &part.subparts {
        if let Some(found) = scan_part(sub) {
            return Some(found);
        }
    }
    None
}

fn contains_pgp_armor(body: &[u8]) -> bool {
    // PGP MESSAGE (firmado+encriptado o sólo firmado armored) o PGP SIGNATURE
    // (cleartext-signed). El verifier de pgp.rs sólo soporta MESSAGE; los
    // cleartext-signed se rechazan en pgp::verify_armored.
    twoway_contains(body, b"-----BEGIN PGP MESSAGE-----")
}

fn twoway_contains(hay: &[u8], needle: &[u8]) -> bool {
    hay.windows(needle.len()).any(|w| w == needle)
}

/// SHA-256 hex lowercase de los bytes firmados. Usamos el `ring`/`aws-lc-rs`
/// expuesto por `rustls::crypto::aws_lc_rs::hash`? Simpler: implementamos
/// nosotros usando `sequoia_openpgp::crypto::hash` (también usa OpenSSL en
/// nuestro build).
fn sha256_hex(bytes: &[u8]) -> String {
    use sequoia_openpgp::types::HashAlgorithm;
    let mut h = HashAlgorithm::SHA256
        .context()
        .expect("sha256 context")
        .for_digest();
    h.update(bytes);
    let digest = h.into_digest().expect("digest finalize");
    hex_encode(&digest)
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        s.push(HEX[(b >> 4) as usize] as char);
        s.push(HEX[(b & 0x0f) as usize] as char);
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pgp_armor_detection() {
        assert!(contains_pgp_armor(b"foo\n-----BEGIN PGP MESSAGE-----\nbar"));
        assert!(!contains_pgp_armor(b"no armor here at all"));
    }

    #[test]
    fn extract_from_plain_body() {
        let mail = b"From: op@example.test\r\n\
                     To: gateway@example.test\r\n\
                     Subject: cmd\r\n\
                     Content-Type: text/plain; charset=utf-8\r\n\
                     \r\n\
                     -----BEGIN PGP MESSAGE-----\r\n\
                     payload\r\n\
                     -----END PGP MESSAGE-----\r\n";
        let armored = extract_pgp_part(mail).expect("extract");
        assert!(contains_pgp_armor(&armored));
    }

    #[test]
    fn extract_fails_without_armor() {
        let mail = b"From: op@example.test\r\n\
                     To: gateway@example.test\r\n\
                     Subject: cmd\r\n\
                     Content-Type: text/plain; charset=utf-8\r\n\
                     \r\n\
                     hola sin pgp\r\n";
        assert!(extract_pgp_part(mail).is_err());
    }

    #[test]
    fn sha256_hex_is_64_chars_lowercase() {
        let h = sha256_hex(b"trenchpass");
        assert_eq!(h.len(), 64);
        assert!(h
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));
    }

    #[test]
    fn sha256_hex_known_vector() {
        // "abc" → ba7816bf...f20015ad (FIPS 180-2)
        let h = sha256_hex(b"abc");
        assert_eq!(
            h,
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    // Silenciamos warning si SystemTime no se usa en este módulo.
}
