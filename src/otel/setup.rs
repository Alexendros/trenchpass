//! Inicializa el pipeline OTLP (gRPC) y el subscriber `tracing` global.
//!
//! Stack OpenTelemetry 0.31 (PR3.3): builder pattern explícito.
//! - `SpanExporter::builder().with_tonic().with_endpoint().build()` → exporter
//! - `SdkTracerProvider::builder().with_batch_exporter(...).with_resource(...).build()`
//! - `tracing_opentelemetry::layer().with_tracer(provider.tracer(name))`
//!
//! `BatchSpanProcessor` corre en su propio hilo (no necesita `rt-tokio`).
//! Para `shutdown()` mantenemos el `SdkTracerProvider` en un `OnceLock`
//! global (el `global::shutdown_tracer_provider()` de 0.26 ya no existe).

use std::sync::OnceLock;

use anyhow::{Context, Result};
use opentelemetry::trace::TracerProvider as _;
use opentelemetry::{global, KeyValue};
use opentelemetry_otlp::{SpanExporter, WithExportConfig};
use opentelemetry_sdk::trace::SdkTracerProvider;
use opentelemetry_sdk::Resource;
use tracing_subscriber::{prelude::*, EnvFilter};

use crate::config::OtelConfig;

static PROVIDER: OnceLock<SdkTracerProvider> = OnceLock::new();

pub fn init(cfg: &OtelConfig, log_level: &str) -> Result<()> {
    let exporter = SpanExporter::builder()
        .with_tonic()
        .with_endpoint(cfg.endpoint.clone())
        .build()
        .context("OTLP SpanExporter build failed")?;

    let resource = build_resource(&cfg.service_name, &cfg.resource_attributes);

    let provider = SdkTracerProvider::builder()
        .with_batch_exporter(exporter)
        .with_resource(resource)
        .build();

    let tracer = provider.tracer(cfg.service_name.clone());

    // Guardamos una referencia para `shutdown()` y otra al global.
    let _ = PROVIDER.set(provider.clone());
    global::set_tracer_provider(provider);

    let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);

    let env_filter = EnvFilter::try_new(log_level)
        .or_else(|_| EnvFilter::try_from_default_env())
        .unwrap_or_else(|_| EnvFilter::new("info"));

    let fmt_layer = tracing_subscriber::fmt::layer()
        .with_target(true)
        .with_thread_ids(false)
        .json();

    tracing_subscriber::registry()
        .with(env_filter)
        .with(fmt_layer)
        .with(otel_layer)
        .try_init()
        .context("global tracing subscriber init failed")?;

    Ok(())
}

/// Drena el batch processor y cierra el exporter. Idempotente.
/// Llamado desde `OtelShutdownGuard::drop` en `main.rs`.
pub fn shutdown() {
    if let Some(p) = PROVIDER.get() {
        // shutdown() es bloqueante; en runtime current-thread podría deadlockear,
        // pero tokio::main por defecto es multi-thread y `OtelShutdownGuard::drop`
        // se ejecuta tras `result.await`, fuera del runtime crítico.
        if let Err(e) = p.shutdown() {
            eprintln!("OTel shutdown error: {e:?}");
        }
    }
}

fn build_resource(service_name: &str, raw: &str) -> Resource {
    let mut attrs = vec![KeyValue::new("service.name", service_name.to_string())];
    for part in raw.split(',').filter(|s| !s.trim().is_empty()) {
        if let Some((k, v)) = part.split_once('=') {
            attrs.push(KeyValue::new(k.trim().to_string(), v.trim().to_string()));
        }
    }
    Resource::builder().with_attributes(attrs).build()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn build_resource_incluye_service_name_y_extras() {
        let r = build_resource("trenchpass", "deployment.environment=staging,team=core");
        let kvs: Vec<_> = r
            .iter()
            .map(|(k, v)| (k.as_ref(), format!("{v}")))
            .collect();
        // Debe contener service.name + las dos extras.
        assert!(kvs
            .iter()
            .any(|(k, v)| *k == "service.name" && v == "trenchpass"));
        assert!(kvs
            .iter()
            .any(|(k, v)| *k == "deployment.environment" && v == "staging"));
        assert!(kvs.iter().any(|(k, v)| *k == "team" && v == "core"));
    }

    #[test]
    fn build_resource_ignora_entradas_malformadas() {
        let r = build_resource("svc", "good=ok,malformed,k2=v2,");
        let kvs: Vec<_> = r.iter().map(|(k, _)| k.as_ref().to_string()).collect();
        assert!(kvs.contains(&"good".to_string()));
        assert!(kvs.contains(&"k2".to_string()));
        assert!(!kvs.contains(&"malformed".to_string()));
    }
}
