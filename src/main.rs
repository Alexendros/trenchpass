//! Punto de entrada del binario `trenchpass`.

use std::net::SocketAddr;
use std::time::Duration;

use anyhow::{Context, Result};
use axum_server::tls_rustls::RustlsAcceptor;
use axum_server::Handle;
use tokio::net::TcpListener;
use tracing::{info, warn};
use trenchpass::config::{Config, TlsMode};
use trenchpass::{otel, transport, AppState};

#[tokio::main]
async fn main() -> Result<()> {
    let config = Config::from_env().context("config inválida")?;
    otel::init(&config.otel, &config.server.log_level).context("otel init")?;
    trenchpass::init_crypto();

    info!(
        target: "trenchpass.boot",
        version = env!("CARGO_PKG_VERSION"),
        env = ?config.env,
        bind = %config.server.bind,
        tls_mode = ?config.tls.mode,
        mtls_required = config.tls.mtls_required,
        "starting TrenchPass gateway"
    );

    // RAII guard: garantiza flush de spans/metrics OTLP aunque cualquier path
    // de `serve_*` (bind error, TLS build error, axum-server error) propague
    // un `Err` antes del final feliz.
    let _otel_guard = OtelShutdownGuard;

    let result: Result<()> = async {
        let state = AppState::build(config).await.context("AppState build")?;
        let bind: SocketAddr = state.config.server.bind;
        let app = transport::router(state.clone());
        match state.config.tls.mode {
            TlsMode::Off => serve_plain(bind, app).await,
            TlsMode::Static | TlsMode::VaultPki => serve_tls(bind, app, &state).await,
        }
    }
    .await;

    if let Err(e) = &result {
        tracing::error!(target: "trenchpass.boot", error = %e, "fatal error · shutting down");
    }
    result
}

struct OtelShutdownGuard;
impl Drop for OtelShutdownGuard {
    fn drop(&mut self) {
        otel::shutdown();
    }
}

async fn serve_plain(bind: SocketAddr, app: axum::Router) -> Result<()> {
    warn!(
        target: "trenchpass.boot",
        "TLS desactivado (TRENCHPASS_TLS_MODE=off) · sólo válido tras proxy reverso terminador"
    );
    let listener = TcpListener::bind(bind)
        .await
        .with_context(|| format!("bind {bind}"))?;
    info!(target: "trenchpass.boot", "listening on {bind} (plain)");
    let serve = axum::serve(listener, app).with_graceful_shutdown(ctrl_c_or_term());
    if let Err(e) = serve.await {
        tracing::error!(target: "trenchpass.boot", "axum serve: {e}");
    }
    Ok(())
}

async fn serve_tls(
    bind: SocketAddr,
    app: axum::Router,
    state: &std::sync::Arc<AppState>,
) -> Result<()> {
    let pki_mount = state.config.vault.pki_mount.clone();
    let handle = transport::mtls::build(&state.config.tls, Some(&state.vault), Some(&pki_mount))
        .await
        .context("TLS build")?;

    let server_handle = Handle::new();
    if let Some(initial) = handle.initial_bundle.clone() {
        transport::mtls::spawn_refresh_loop(
            handle.config.clone(),
            state.vault.clone(),
            pki_mount,
            state.config.tls.clone(),
            initial,
        );
    }

    tokio::spawn(graceful_shutdown(server_handle.clone()));

    info!(target: "trenchpass.boot", "listening on {bind} (TLS+mTLS peer cert wired)");
    // Custom acceptor: RustlsAcceptor → PeerCertAcceptor (inyecta peer cert
    // validado por WebPkiClientVerifier en Request::extensions).
    let acceptor = transport::PeerCertAcceptor::new(RustlsAcceptor::new(handle.config));
    axum_server::bind(bind)
        .acceptor(acceptor)
        .handle(server_handle)
        .serve(app.into_make_service())
        .await
        .context("axum_server bind+serve")?;
    Ok(())
}

async fn graceful_shutdown(handle: Handle) {
    ctrl_c_or_term().await;
    info!(target: "trenchpass.boot", "shutdown signal · graceful 30s");
    handle.graceful_shutdown(Some(Duration::from_secs(30)));
}

async fn ctrl_c_or_term() {
    use tokio::signal::unix::{signal, SignalKind};

    let ctrl_c = async {
        tokio::signal::ctrl_c().await.ok();
    };
    let term = async {
        let mut s = signal(SignalKind::terminate()).expect("install SIGTERM handler");
        s.recv().await;
    };
    tokio::select! {
        _ = ctrl_c => info!(target: "trenchpass.boot", "ctrl-c received"),
        _ = term => info!(target: "trenchpass.boot", "SIGTERM received"),
    }
}
