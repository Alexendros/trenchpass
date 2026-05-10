//! Inicializa el pipeline OTLP (gRPC) y el subscriber `tracing` global.
//!
//! En PR1 montamos solo traces; metrics y logs se añaden en PR2 cuando
//! el otel-collector tenga sus tres pipelines confirmados.

use anyhow::{Context, Result};
use opentelemetry::{global, trace::TracerProvider as _, KeyValue};
use opentelemetry_otlp::WithExportConfig;
use opentelemetry_sdk::{runtime, trace::Config as TraceConfig, Resource};
use tracing_subscriber::{prelude::*, EnvFilter};

use crate::config::OtelConfig;

pub fn init(cfg: &OtelConfig, log_level: &str) -> Result<()> {
    let resource = Resource::new(parse_resource_attrs(
        &cfg.service_name,
        &cfg.resource_attributes,
    ));

    // En opentelemetry-otlp 0.26, `install_batch` devuelve `TracerProvider`,
    // no `Tracer`. Lo registramos globalmente y derivamos un `Tracer` con
    // nombre de scope para alimentar a tracing-opentelemetry.
    let provider = opentelemetry_otlp::new_pipeline()
        .tracing()
        .with_exporter(
            opentelemetry_otlp::new_exporter()
                .tonic()
                .with_endpoint(cfg.endpoint.clone()),
        )
        .with_trace_config(TraceConfig::default().with_resource(resource))
        .install_batch(runtime::Tokio)
        .context("OTLP tracer install_batch failed")?;

    let tracer = provider.tracer(cfg.service_name.clone());
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

pub fn shutdown() {
    global::shutdown_tracer_provider();
}

fn parse_resource_attrs(service_name: &str, raw: &str) -> Vec<KeyValue> {
    let mut out = vec![KeyValue::new("service.name", service_name.to_string())];
    for part in raw.split(',').filter(|s| !s.trim().is_empty()) {
        if let Some((k, v)) = part.split_once('=') {
            out.push(KeyValue::new(k.trim().to_string(), v.trim().to_string()));
        }
    }
    out
}
