//! Namespace `stripe.*` · cliente HTTP contra `api.stripe.com/v1`.
//!
//! Tools registradas en PR2c:
//! - `stripe.list_customers`   · GET /customers · paginación `limit` (1..100)
//! - `stripe.retrieve_balance` · GET /balance   · saldo disponible/pendiente
//!
//! El secret se carga desde Vault en `stripe/api_key` (campo `secret_key`).
//! Stripe autentica con HTTP Basic, usuario = secret, password vacío.

use async_trait::async_trait;
use reqwest::Client;
use serde_json::Value;
use tracing::instrument;

use super::{RegistryBuilder, ToolContext, ToolDef, ToolHandler};
use crate::error::{Error, Result};

const VAULT_PATH: &str = "stripe/api_key";
const SECRET_FIELD: &str = "secret_key";
const STRIPE_API_VERSION: &str = "2025-04-30.basil";

pub fn register(b: &mut RegistryBuilder, base_url: &str) {
    let http = Client::builder()
        .user_agent(concat!("trenchpass/", env!("CARGO_PKG_VERSION")))
        .build()
        .expect("reqwest client");

    b.register(
        ToolDef {
            id: "stripe.list_customers".into(),
            namespace: "stripe",
            description: "Lista de clientes Stripe (GET /v1/customers).",
        },
        ListCustomers {
            http: http.clone(),
            base_url: base_url.to_string(),
        },
    );

    b.register(
        ToolDef {
            id: "stripe.retrieve_balance".into(),
            namespace: "stripe",
            description: "Saldo disponible y pendiente (GET /v1/balance).",
        },
        RetrieveBalance {
            http,
            base_url: base_url.to_string(),
        },
    );
}

struct ListCustomers {
    http: Client,
    base_url: String,
}

struct RetrieveBalance {
    http: Client,
    base_url: String,
}

async fn load_secret(ctx: &ToolContext<'_>) -> Result<String> {
    let secret = ctx.vault.secret(VAULT_PATH).await?;
    extract_field(&secret.data, SECRET_FIELD)
        .ok_or_else(|| Error::Vault(format!("campo `{SECRET_FIELD}` ausente en {VAULT_PATH}")))
}

fn extract_field(data: &Value, field: &str) -> Option<String> {
    data.get("data")
        .and_then(|d| d.get(field))
        .or_else(|| data.get(field))
        .and_then(Value::as_str)
        .map(str::to_string)
}

fn map_status(status: reqwest::StatusCode, body: &str) -> Error {
    Error::Upstream(format!("stripe {status}: {body}"))
}

async fn parse_json(body: String, op: &str) -> Result<Value> {
    serde_json::from_str(&body).map_err(|e| Error::Upstream(format!("stripe.{op} parse: {e}")))
}

#[async_trait]
impl ToolHandler for ListCustomers {
    #[instrument(skip(self, ctx, params), fields(tool = "stripe.list_customers"))]
    async fn invoke(&self, ctx: &ToolContext<'_>, params: Value) -> Result<Value> {
        let limit = params
            .get("limit")
            .and_then(Value::as_u64)
            .unwrap_or(10)
            .clamp(1, 100);

        let secret = load_secret(ctx).await?;
        let url = format!("{}/customers", self.base_url.trim_end_matches('/'));

        let resp = self
            .http
            .get(url)
            .basic_auth(&secret, Some(""))
            .header("Stripe-Version", STRIPE_API_VERSION)
            .query(&[("limit", limit.to_string())])
            .send()
            .await
            .map_err(|e| Error::Upstream(format!("stripe.list_customers send: {e}")))?;

        let status = resp.status();
        let body = resp
            .text()
            .await
            .map_err(|e| Error::Upstream(format!("stripe.list_customers read: {e}")))?;

        if !status.is_success() {
            return Err(map_status(status, &body));
        }
        parse_json(body, "list_customers").await
    }
}

#[async_trait]
impl ToolHandler for RetrieveBalance {
    #[instrument(skip(self, ctx, _params), fields(tool = "stripe.retrieve_balance"))]
    async fn invoke(&self, ctx: &ToolContext<'_>, _params: Value) -> Result<Value> {
        let secret = load_secret(ctx).await?;
        let url = format!("{}/balance", self.base_url.trim_end_matches('/'));

        let resp = self
            .http
            .get(url)
            .basic_auth(&secret, Some(""))
            .header("Stripe-Version", STRIPE_API_VERSION)
            .send()
            .await
            .map_err(|e| Error::Upstream(format!("stripe.retrieve_balance send: {e}")))?;

        let status = resp.status();
        let body = resp
            .text()
            .await
            .map_err(|e| Error::Upstream(format!("stripe.retrieve_balance read: {e}")))?;

        if !status.is_success() {
            return Err(map_status(status, &body));
        }
        parse_json(body, "retrieve_balance").await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn extract_kv2_envelope() {
        let v = json!({"data": {"secret_key": "sk_test_123"}});
        assert_eq!(
            extract_field(&v, "secret_key").as_deref(),
            Some("sk_test_123")
        );
    }

    #[test]
    fn limit_clamp_bounds() {
        // Sanity sobre el rango Stripe documenta: 1..=100.
        assert_eq!(0u64.clamp(1, 100), 1);
        assert_eq!(500u64.clamp(1, 100), 100);
    }

    #[tokio::test]
    async fn list_customers_uses_basic_auth_and_version_header() {
        use wiremock::matchers::{header, method, path, query_param};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        let body = json!({"object": "list", "data": [{"id": "cus_1"}]});
        Mock::given(method("GET"))
            .and(path("/customers"))
            .and(query_param("limit", "25"))
            // Basic auth con sk_test_123 vacío → base64("sk_test_123:") = c2tfdGVzdF8xMjM6
            .and(header("Authorization", "Basic c2tfdGVzdF8xMjM6"))
            .and(header("Stripe-Version", STRIPE_API_VERSION))
            .respond_with(ResponseTemplate::new(200).set_body_json(body.clone()))
            .expect(1)
            .mount(&server)
            .await;

        let mut b = RegistryBuilder::default();
        register(&mut b, &server.uri());
        let registry = b.finish();
        let (_, handler) = registry.get("stripe.list_customers").expect("registered");

        let vault = crate::vault::VaultClient::test_with_secret(
            VAULT_PATH,
            json!({"data": {"secret_key": "sk_test_123"}}),
        );
        let consumer = crate::auth::Consumer::dev("test");
        let ctx = ToolContext {
            consumer: &consumer,
            vault: &vault,
        };

        let out = handler
            .invoke(&ctx, json!({"limit": 25}))
            .await
            .expect("ok");
        assert_eq!(out, body);
    }

    #[tokio::test]
    async fn retrieve_balance_propagates_401() {
        use wiremock::matchers::{method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/balance"))
            .respond_with(ResponseTemplate::new(401).set_body_string(
                r#"{"error":{"type":"invalid_request_error","message":"Invalid API Key"}}"#,
            ))
            .expect(1)
            .mount(&server)
            .await;

        let mut b = RegistryBuilder::default();
        register(&mut b, &server.uri());
        let registry = b.finish();
        let (_, handler) = registry.get("stripe.retrieve_balance").expect("registered");

        let vault = crate::vault::VaultClient::test_with_secret(
            VAULT_PATH,
            json!({"data": {"secret_key": "bad"}}),
        );
        let consumer = crate::auth::Consumer::dev("test");
        let ctx = ToolContext {
            consumer: &consumer,
            vault: &vault,
        };

        let err = handler
            .invoke(&ctx, json!({}))
            .await
            .expect_err("401 esperado");
        let msg = err.to_string();
        assert!(msg.contains("401"), "msg: {msg}");
        assert!(msg.contains("Invalid API Key"), "msg: {msg}");
    }
}
