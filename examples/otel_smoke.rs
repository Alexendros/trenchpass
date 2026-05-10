//! Smoke OTel: inicializa el pipeline OTLP/gRPC y emite spans `tracing` →
//! verifica que llegan al collector escuchando en :4317.
//!
//! Pre-requisitos:
//!   `otelcol-contrib --config /tmp/otelcol-config.yaml` (debug exporter
//!   stdout · ver `examples/otelcol-config.yaml`).
//!
//! Uso:
//!   `OTEL_EXPORTER_OTLP_ENDPOINT=http://127.0.0.1:4317 cargo run --example otel_smoke`

use std::time::Duration;

use anyhow::Result;
use trenchpass::config::OtelConfig;
use trenchpass::otel;

#[tokio::main]
async fn main() -> Result<()> {
    let cfg = OtelConfig {
        endpoint: std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT")
            .unwrap_or_else(|_| "http://127.0.0.1:4317".into()),
        service_name: "trenchpass-otel-smoke".into(),
        resource_attributes: "deployment.environment=smoke,team=core".into(),
    };

    println!("[smoke] init OTel pipeline · endpoint={}", cfg.endpoint);
    otel::init(&cfg, "info").expect("otel::init");

    // Emite varios spans con atributos para que el collector los muestre.
    {
        let _enter = tracing::info_span!("smoke.outer", req_id = "abc-123").entered();
        tracing::info!(target: "smoke", "outer info event");
        {
            let _inner =
                tracing::info_span!("smoke.inner", path = "/healthz", status = 200).entered();
            tracing::info!(target: "smoke", "inner info event");
            tracing::warn!(target: "smoke", "inner warn event");
        }
    }

    // Forzar flush dando margen al BatchSpanProcessor (default scheduled_delay 5s).
    println!("[smoke] esperando 7s para batch flush…");
    tokio::time::sleep(Duration::from_secs(7)).await;

    println!("[smoke] llamando otel::shutdown");
    otel::shutdown();
    println!("[smoke] FIN · revisar stdout del collector para confirmar spans");
    Ok(())
}
