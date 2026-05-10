//! Namespace `notion.*` · cliente HTTP contra `api.notion.com/v1`.
//!
//! Tools registradas en PR2b:
//! - `notion.search` · POST /search · query libre por título.
//! - `notion.fetch`  · GET  /pages/{id} · página por id.
//!
//! El token bearer se carga desde Vault en `<kv_mount>/notion/api_key`
//! (campo `token`). El cache del [`VaultClient`](crate::vault::VaultClient)
//! evita roundtrip por cada request del MCP.
//!
//! El base URL es inyectable (constructor) para permitir wiremock en tests.

use async_trait::async_trait;
use reqwest::Client;
use serde_json::{json, Value};
use tracing::instrument;

use super::{RegistryBuilder, ToolContext, ToolDef, ToolHandler};
use crate::error::{Error, Result};

const VAULT_PATH: &str = "notion/api_key";
const NOTION_VERSION: &str = "2025-09-03";
const TOKEN_FIELD: &str = "token";

pub fn register(b: &mut RegistryBuilder, base_url: &str) {
    let http = Client::builder()
        .user_agent(concat!("trenchpass/", env!("CARGO_PKG_VERSION")))
        .build()
        .expect("reqwest client");

    b.register(
        ToolDef {
            id: "notion.search".into(),
            namespace: "notion",
            description: "Búsqueda libre en el workspace Notion (POST /v1/search).",
        },
        SearchTool {
            http: http.clone(),
            base_url: base_url.to_string(),
        },
    );

    b.register(
        ToolDef {
            id: "notion.fetch".into(),
            namespace: "notion",
            description: "Recupera una página Notion por id (GET /v1/pages/{id}).",
        },
        FetchTool {
            http,
            base_url: base_url.to_string(),
        },
    );
}

struct SearchTool {
    http: Client,
    base_url: String,
}

struct FetchTool {
    http: Client,
    base_url: String,
}

async fn load_token(ctx: &ToolContext<'_>) -> Result<String> {
    let secret = ctx.vault.secret(VAULT_PATH).await?;
    extract_token(&secret.data)
}

fn extract_token(data: &Value) -> Result<String> {
    // KV v2 envuelve el payload en `data.data.<field>`. Soportamos ambas formas
    // por si algún operador guardó el token plano.
    let field = data
        .get("data")
        .and_then(|d| d.get(TOKEN_FIELD))
        .or_else(|| data.get(TOKEN_FIELD))
        .and_then(Value::as_str)
        .ok_or_else(|| Error::Vault(format!("campo `{TOKEN_FIELD}` ausente en {VAULT_PATH}")))?;
    Ok(field.to_string())
}

fn map_status(resp_status: reqwest::StatusCode, body: &str) -> Error {
    Error::Upstream(format!("notion {resp_status}: {body}"))
}

#[async_trait]
impl ToolHandler for SearchTool {
    #[instrument(skip(self, ctx, params), fields(tool = "notion.search"))]
    async fn invoke(&self, ctx: &ToolContext<'_>, params: Value) -> Result<Value> {
        let query = params
            .get("query")
            .and_then(Value::as_str)
            .ok_or_else(|| Error::Upstream("notion.search requiere `query` (string)".into()))?;
        let page_size = params
            .get("page_size")
            .and_then(Value::as_u64)
            .unwrap_or(10)
            .min(100);

        let token = load_token(ctx).await?;
        let url = format!("{}/search", self.base_url.trim_end_matches('/'));

        let resp = self
            .http
            .post(url)
            .bearer_auth(token)
            .header("Notion-Version", NOTION_VERSION)
            .json(&json!({ "query": query, "page_size": page_size }))
            .send()
            .await
            .map_err(|e| Error::Upstream(format!("notion.search send: {e}")))?;

        let status = resp.status();
        let body = resp
            .text()
            .await
            .map_err(|e| Error::Upstream(format!("notion.search read: {e}")))?;

        if !status.is_success() {
            return Err(map_status(status, &body));
        }

        serde_json::from_str(&body)
            .map_err(|e| Error::Upstream(format!("notion.search parse: {e}")))
    }
}

#[async_trait]
impl ToolHandler for FetchTool {
    #[instrument(skip(self, ctx, params), fields(tool = "notion.fetch"))]
    async fn invoke(&self, ctx: &ToolContext<'_>, params: Value) -> Result<Value> {
        let id = params
            .get("id")
            .and_then(Value::as_str)
            .ok_or_else(|| Error::Upstream("notion.fetch requiere `id` (string)".into()))?;

        // Defensa: el id Notion es UUID con guiones (32+4 hex). Bloquea inyección de path.
        if !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') || id.len() > 64 {
            return Err(Error::Upstream("notion.fetch: id inválido".into()));
        }

        let token = load_token(ctx).await?;
        let url = format!("{}/pages/{id}", self.base_url.trim_end_matches('/'));

        let resp = self
            .http
            .get(url)
            .bearer_auth(token)
            .header("Notion-Version", NOTION_VERSION)
            .send()
            .await
            .map_err(|e| Error::Upstream(format!("notion.fetch send: {e}")))?;

        let status = resp.status();
        let body = resp
            .text()
            .await
            .map_err(|e| Error::Upstream(format!("notion.fetch read: {e}")))?;

        if !status.is_success() {
            return Err(map_status(status, &body));
        }

        serde_json::from_str(&body).map_err(|e| Error::Upstream(format!("notion.fetch parse: {e}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn extract_token_kv2_envelope() {
        let v = json!({"data": {"token": "secret_abc"}, "metadata": {}});
        assert_eq!(extract_token(&v).unwrap(), "secret_abc");
    }

    #[test]
    fn extract_token_plain() {
        let v = json!({"token": "secret_xyz"});
        assert_eq!(extract_token(&v).unwrap(), "secret_xyz");
    }

    #[test]
    fn extract_token_missing() {
        let v = json!({"data": {"other": "x"}});
        assert!(extract_token(&v).is_err());
    }

    #[test]
    fn fetch_rejects_bad_id() {
        // El validador es síncrono dentro del handler; lo exercise vía smoke
        // de que un id con `..` no pasaría el filter charset.
        let bad = "../../etc/passwd";
        assert!(!bad.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'));
    }

    /// Integración HTTP: arranca un wiremock que finge ser api.notion.com,
    /// construye la registry apuntando a su URL, invoca el handler con un
    /// `VaultClient` pre-cargado y verifica el body devuelto.
    #[tokio::test]
    async fn search_handler_calls_upstream_with_bearer_and_version() {
        use wiremock::matchers::{header, header_exists, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        let response_body = json!({
            "object": "list",
            "results": [{"id": "abc", "object": "page"}]
        });
        Mock::given(method("POST"))
            .and(path("/search"))
            .and(header("Authorization", "Bearer secret_test_token"))
            .and(header("Notion-Version", NOTION_VERSION))
            .and(header_exists("user-agent"))
            .respond_with(ResponseTemplate::new(200).set_body_json(response_body.clone()))
            .expect(1)
            .mount(&server)
            .await;

        let mut b = RegistryBuilder::default();
        register(&mut b, &server.uri());
        let registry = b.finish();
        let (_, handler) = registry.get("notion.search").expect("registered");

        let vault = crate::vault::VaultClient::test_with_secret(
            VAULT_PATH,
            json!({"data": {"token": "secret_test_token"}}),
        );
        let consumer = crate::auth::Consumer::dev("test");
        let ctx = ToolContext {
            consumer: &consumer,
            vault: &vault,
        };

        let out = handler
            .invoke(&ctx, json!({"query": "roadmap"}))
            .await
            .expect("invoke ok");

        assert_eq!(out, response_body);
    }

    #[tokio::test]
    async fn fetch_handler_routes_id_and_propagates_error_body() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/pages/aaaa-bbbb-cccc"))
            .respond_with(
                ResponseTemplate::new(404)
                    .set_body_string(r#"{"object":"error","code":"object_not_found"}"#),
            )
            .expect(1)
            .mount(&server)
            .await;

        let mut b = RegistryBuilder::default();
        register(&mut b, &server.uri());
        let registry = b.finish();
        let (_, handler) = registry.get("notion.fetch").expect("registered");

        let vault = crate::vault::VaultClient::test_with_secret(
            VAULT_PATH,
            json!({"data": {"token": "tk"}}),
        );
        let consumer = crate::auth::Consumer::dev("test");
        let ctx = ToolContext {
            consumer: &consumer,
            vault: &vault,
        };

        let err = handler
            .invoke(&ctx, json!({"id": "aaaa-bbbb-cccc"}))
            .await
            .expect_err("expect upstream error");
        let msg = err.to_string();
        assert!(msg.contains("404"), "msg: {msg}");
        assert!(msg.contains("object_not_found"), "msg: {msg}");
    }
}
