//! Wiring del peer cert mTLS validado por rustls hacia la capa de aplicación.
//!
//! El handshake de [`axum_server::tls_rustls::RustlsAcceptor`] ya valida el
//! cert cliente vía `WebPkiClientVerifier` (cuando está configurado), pero
//! NO expone el cert al `Service` axum (issue programatik29/axum-server#162).
//!
//! Este módulo provee:
//! - [`PeerCertificate`]: newtype clonable barato (`Arc<CertificateDer>`),
//!   insertado en `Request::extensions()` por cada request de la conexión.
//! - [`PeerCertAcceptor`]: wrapper genérico sobre cualquier
//!   [`axum_server::accept::Accept`] cuyo `Stream` sea un
//!   [`tokio_rustls::server::TlsStream`]. Captura el peer cert tras el
//!   handshake e inyecta un layer `AddPeerCertService` que lo añade a las
//!   extensions de cada request servida en esa conexión.

use std::sync::Arc;
use std::task::{Context, Poll};

use axum_server::accept::Accept;
use futures_util::future::BoxFuture;
use http::Request;
use rustls_pki_types::CertificateDer;
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_rustls::server::TlsStream;
use tower::Service;

/// Cert presentado por el cliente durante el handshake mTLS.
/// Insertado en `Request::extensions()` por [`PeerCertAcceptor`].
#[derive(Clone, Debug)]
pub struct PeerCertificate(pub Arc<CertificateDer<'static>>);

/// Acceptor que envuelve otro `Accept` (típicamente `RustlsAcceptor`) para
/// extraer el peer cert tras el handshake e inyectarlo en cada request.
#[derive(Clone, Debug, Default)]
pub struct PeerCertAcceptor<A> {
    inner: A,
}

impl<A> PeerCertAcceptor<A> {
    pub fn new(inner: A) -> Self {
        Self { inner }
    }
}

impl<A, I, S> Accept<I, S> for PeerCertAcceptor<A>
where
    A: Accept<I, S, Stream = TlsStream<I>>,
    A::Future: Send + 'static,
    A::Service: Send + 'static,
    I: AsyncRead + AsyncWrite + Unpin,
{
    type Stream = A::Stream;
    type Service = AddPeerCertService<A::Service>;
    type Future = BoxFuture<'static, std::io::Result<(Self::Stream, Self::Service)>>;

    fn accept(&self, stream: I, service: S) -> Self::Future {
        let inner_fut = self.inner.accept(stream, service);
        Box::pin(async move {
            let (tls_stream, service) = inner_fut.await?;
            // peer_certificates() devuelve Option<&[CertificateDer<'_>]>.
            // Clonamos a 'static via `to_vec` (CertificateDer Deref a [u8]).
            let peer_cert = tls_stream
                .get_ref()
                .1
                .peer_certificates()
                .and_then(|certs| certs.first())
                .map(|cert| {
                    let bytes: Vec<u8> = cert.as_ref().to_vec();
                    PeerCertificate(Arc::new(CertificateDer::from(bytes)))
                });
            let svc = AddPeerCertService {
                inner: service,
                peer_cert: Arc::new(peer_cert),
            };
            Ok((tls_stream, svc))
        })
    }
}

/// Service wrapper que inyecta `PeerCertificate` (si existe) en cada request.
/// Clone barato (Arc internamente). `Send + 'static` requerido por
/// `axum_server::service::SendService`.
#[derive(Clone)]
pub struct AddPeerCertService<S> {
    inner: S,
    peer_cert: Arc<Option<PeerCertificate>>,
}

impl<S, B> Service<Request<B>> for AddPeerCertService<S>
where
    S: Service<Request<B>>,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = S::Future;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: Request<B>) -> Self::Future {
        if let Some(cert) = self.peer_cert.as_ref() {
            req.extensions_mut().insert(cert.clone());
        }
        self.inner.call(req)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn peer_certificate_is_clone_cheap() {
        let cert = CertificateDer::from(vec![1, 2, 3]);
        let pc = PeerCertificate(Arc::new(cert));
        let pc2 = pc.clone();
        assert_eq!(pc.0.as_ref(), pc2.0.as_ref());
    }
}
