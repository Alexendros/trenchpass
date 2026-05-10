//! Helpers compartidos para handlers de namespace · evitan duplicar el patrón
//! "extraer token del KV v2, lanzar GET con Bearer o custom-header, deserializar JSON".

use reqwest::Client;
use serde_json::Value;
use tracing::instrument;

use super::ToolContext;
use crate::error::{Error, Result};

/// Esquema de autenticación que cada namespace usa contra su upstream.
/// `Bearer(token)` → header `Authorization: Bearer <token>`.
/// `Header { name, value }` → header arbitrario (caso n8n `X-N8N-API-KEY`,
/// docuseal `X-Auth-Token`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AuthScheme {
    Bearer(String),
    Header { name: &'static str, value: String },
}

/// Extrae un campo string del JSON de un secret KV v2. Soporta tanto el envelope
/// `{data: {<field>}}` como el cuerpo plano `{<field>}` (operador legacy).
pub fn extract_string_field(data: &Value, field: &str, vault_path: &str) -> Result<String> {
    data.get("data")
        .and_then(|d| d.get(field))
        .or_else(|| data.get(field))
        .and_then(Value::as_str)
        .map(|s| s.to_string())
        .ok_or_else(|| Error::Vault(format!("campo `{field}` ausente en {vault_path}")))
}

/// Carga un secret de Vault y devuelve el campo `field` como String.
pub async fn load_secret_field(
    ctx: &ToolContext<'_>,
    vault_path: &str,
    field: &str,
) -> Result<String> {
    let secret = ctx.vault.secret(vault_path).await?;
    extract_string_field(&secret.data, field, vault_path)
}

/// `GET <url>` con autenticación según `AuthScheme`. Devuelve JSON parseado o
/// `Error::Upstream` con cuerpo si status no es 2xx.
#[instrument(skip(http, auth), fields(tool_ns = ns))]
pub async fn auth_get_json(
    http: &Client,
    ns: &'static str,
    url: &str,
    auth: &AuthScheme,
    extra_headers: &[(&str, &str)],
) -> Result<Value> {
    let mut req = http.get(url);
    req = match auth {
        AuthScheme::Bearer(t) => req.bearer_auth(t),
        AuthScheme::Header { name, value } => req.header(*name, value),
    };
    for (k, v) in extra_headers {
        req = req.header(*k, *v);
    }
    let resp = req
        .send()
        .await
        .map_err(|e| Error::Upstream(format!("{ns} send: {e}")))?;
    let status = resp.status();
    let body = resp
        .text()
        .await
        .map_err(|e| Error::Upstream(format!("{ns} read: {e}")))?;
    if !status.is_success() {
        return Err(Error::Upstream(format!("{ns} {status}: {body}")));
    }
    serde_json::from_str(&body).map_err(|e| Error::Upstream(format!("{ns} parse: {e}")))
}

/// Wrapper retro-compat para call sites que sólo usan Bearer.
/// Permite no tocar los handlers ya migrados.
/// `name = "bearer_get_json"` mantiene el nombre original del span para no
/// romper filters/dashboards existentes que filtraban por ese nombre.
#[instrument(name = "bearer_get_json", skip(http, token), fields(tool_ns = ns))]
pub async fn bearer_get_json(
    http: &Client,
    ns: &'static str,
    url: &str,
    token: &str,
    extra_headers: &[(&str, &str)],
) -> Result<Value> {
    auth_get_json(
        http,
        ns,
        url,
        &AuthScheme::Bearer(token.to_string()),
        extra_headers,
    )
    .await
}

/// User-Agent común para todos los reqwest clients del gateway.
pub fn user_agent() -> String {
    format!("trenchpass/{}", env!("CARGO_PKG_VERSION"))
}

/// Construye el `reqwest::Client` con UA estándar (sirve para el reuso entre
/// namespaces nuevos · los originales mantienen su builder local).
pub fn http_client() -> Client {
    Client::builder()
        .user_agent(user_agent())
        .build()
        .expect("reqwest client")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn extract_string_field_kv2_envelope() {
        let v = json!({"data": {"token": "abc"}, "metadata": {}});
        assert_eq!(
            extract_string_field(&v, "token", "ns/secret").unwrap(),
            "abc"
        );
    }

    #[test]
    fn extract_string_field_plano() {
        let v = json!({"api_key": "xyz"});
        assert_eq!(extract_string_field(&v, "api_key", "ns/x").unwrap(), "xyz");
    }

    #[test]
    fn extract_string_field_missing() {
        let v = json!({"data": {"other": "x"}});
        let err = extract_string_field(&v, "token", "ns/y").unwrap_err();
        assert!(err.to_string().contains("campo `token` ausente"));
    }

    #[tokio::test]
    async fn bearer_get_json_propaga_status_no_2xx() {
        use wiremock::matchers::{header, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/x"))
            .and(header("Authorization", "Bearer tk"))
            .respond_with(ResponseTemplate::new(403).set_body_string("forbidden"))
            .expect(1)
            .mount(&server)
            .await;

        let client = http_client();
        let url = format!("{}/x", server.uri());
        let err = bearer_get_json(&client, "test_ns", &url, "tk", &[])
            .await
            .unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("403"), "msg: {msg}");
        assert!(msg.contains("forbidden"), "msg: {msg}");
    }

    #[tokio::test]
    async fn bearer_get_json_extra_headers_se_envian() {
        use wiremock::matchers::{header, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/y"))
            .and(header("X-Test", "value-1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"ok": true})))
            .expect(1)
            .mount(&server)
            .await;

        let client = http_client();
        let url = format!("{}/y", server.uri());
        let body = bearer_get_json(&client, "test_ns", &url, "tk", &[("X-Test", "value-1")])
            .await
            .unwrap();
        assert_eq!(body, json!({"ok": true}));
    }

    #[tokio::test]
    async fn auth_get_json_envia_header_custom() {
        use wiremock::matchers::{header, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/z"))
            .and(header("X-Custom", "my-key"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"data": "ok"})))
            .expect(1)
            .mount(&server)
            .await;

        let client = http_client();
        let url = format!("{}/z", server.uri());
        let body = auth_get_json(
            &client,
            "test_ns",
            &url,
            &AuthScheme::Header {
                name: "X-Custom",
                value: "my-key".into(),
            },
            &[],
        )
        .await
        .unwrap();
        assert_eq!(body, json!({"data": "ok"}));
    }

    #[tokio::test]
    async fn auth_get_json_envia_bearer() {
        use wiremock::matchers::{header, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path("/w"))
            .and(header("Authorization", "Bearer xyz"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"result": 1})))
            .expect(1)
            .mount(&server)
            .await;

        let client = http_client();
        let url = format!("{}/w", server.uri());
        let body = auth_get_json(
            &client,
            "test_ns",
            &url,
            &AuthScheme::Bearer("xyz".into()),
            &[],
        )
        .await
        .unwrap();
        assert_eq!(body, json!({"result": 1}));
    }
}
