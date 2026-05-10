//! mTLS acceptor sobre rustls (PR3).
//!
//! En PR1 dejamos la firma de la función prevista; el binario actual escucha
//! HTTP plano detrás de Traefik, que termina TLS y reenvía cert via header.

use std::path::Path;

use anyhow::Result;

#[allow(dead_code)]
pub struct TlsAcceptorConfig<'a> {
    pub cert: &'a Path,
    pub key: &'a Path,
    pub client_ca: Option<&'a Path>,
    pub require_client_cert: bool,
}

#[allow(dead_code)]
pub fn build(_cfg: TlsAcceptorConfig<'_>) -> Result<tokio_rustls::TlsAcceptor> {
    // PR3: cargar PEMs, construir `ServerConfig` con `WebPkiClientVerifier`
    // si `require_client_cert`, retornar acceptor.
    anyhow::bail!("mtls::build será implementado en PR3")
}
