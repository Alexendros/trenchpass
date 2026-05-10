//! Helpers compartidos para handlers de namespace · evitan duplicar el patrón
//! "extraer token del KV v2, lanzar GET con Bearer, deserializar JSON".

use reqwest::Client;
use serde_json::Value;
use tracing::instrument;

use super::ToolContext;
use crate::error::{Error, Result};

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

/// `GET <url>` con `Authorization: Bearer <token>`. Devuelve el JSON parseado o
/// un `Error::Upstream` con el cuerpo de la respuesta si el status no es 2xx.
#[instrument(skip(http, token), fields(tool_ns = ns))]
pub async fn bearer_get_json(
    http: &Client,
    ns: &'static str,
    url: &str,
    token: &str,
    extra_headers: &[(&str, &str)],
) -> Result<Value> {
    let mut req = http.get(url).bearer_auth(token);
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
}
