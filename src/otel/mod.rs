//! OpenTelemetry init: tracer + métricas → otel-collector → SigNoz.

pub mod setup;

pub use setup::{init, shutdown};
