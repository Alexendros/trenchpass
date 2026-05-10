//! Punto de entrada del binario `trenchpass`.

use std::net::SocketAddr;

use anyhow::{Context, Result};
use tokio::net::TcpListener;
use tracing::info;
use trenchpass::{config::Config, otel, transport, AppState};

#[tokio::main]
async fn main() -> Result<()> {
    let config = Config::from_env().context("config inválida")?;
    otel::init(&config.otel, &config.server.log_level).context("otel init")?;

    info!(
        target: "trenchpass.boot",
        version = env!("CARGO_PKG_VERSION"),
        env = ?config.env,
        bind = %config.server.bind,
        "starting TrenchPass gateway"
    );

    let state = AppState::build(config).await.context("AppState build")?;
    let bind: SocketAddr = state.config.server.bind;
    let app = transport::router(state.clone());

    let listener = TcpListener::bind(bind)
        .await
        .with_context(|| format!("bind {bind}"))?;
    info!(target: "trenchpass.boot", "listening on {bind}");

    let serve = axum::serve(listener, app).with_graceful_shutdown(shutdown_signal());

    if let Err(e) = serve.await {
        tracing::error!(target: "trenchpass.boot", "axum serve: {e}");
    }

    otel::shutdown();
    Ok(())
}

async fn shutdown_signal() {
    use tokio::signal::unix::{signal, SignalKind};

    let ctrl_c = async {
        tokio::signal::ctrl_c().await.ok();
    };
    let term = async {
        let mut s = signal(SignalKind::terminate()).expect("install SIGTERM handler");
        s.recv().await;
    };

    tokio::select! {
        _ = ctrl_c => info!(target: "trenchpass.boot", "ctrl-c · shutting down"),
        _ = term => info!(target: "trenchpass.boot", "SIGTERM · shutting down"),
    }
}
