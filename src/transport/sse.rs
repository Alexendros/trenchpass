//! Router HTTP/SSE básico. PR2 lo reemplaza por el handler `rmcp` streamable-http.

use std::sync::Arc;

use axum::{
    extract::{Path, State},
    http::StatusCode,
    middleware,
    routing::{get, post},
    Extension, Json, Router,
};
use serde_json::{json, Value};

use crate::auth::Consumer;
use crate::error::Result;
use crate::security::auth_middleware;
use crate::tools::dispatch;
use crate::AppState;

pub fn router(state: Arc<AppState>) -> Router {
    let public = Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz));

    let protected = Router::new()
        .route("/tool/:name", post(invoke_tool))
        .route("/tools", get(list_tools))
        .route_layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ));

    Router::new()
        .merge(public)
        .merge(protected)
        .with_state(state)
}

async fn healthz() -> &'static str {
    "ok"
}

async fn readyz(State(state): State<Arc<AppState>>) -> (StatusCode, Json<Value>) {
    // PR2: ping a Vault y Postgres real.
    let _ = state;
    (
        StatusCode::OK,
        Json(json!({ "status": "ready", "version": env!("CARGO_PKG_VERSION") })),
    )
}

async fn list_tools(State(state): State<Arc<AppState>>) -> Json<Value> {
    Json(json!({
        "namespaces": state.tools.namespaces(),
        "tools": state.tools.list(),
    }))
}

async fn invoke_tool(
    State(state): State<Arc<AppState>>,
    Path(name): Path<String>,
    Extension(consumer): Extension<Consumer>,
    Json(params): Json<Value>,
) -> Result<Json<Value>> {
    let out = dispatch(
        &state.tools,
        &state.audit,
        &state.vault,
        &consumer,
        &name,
        params,
    )
    .await?;
    Ok(Json(out))
}
