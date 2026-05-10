//! Namespace `github.*` · cliente HTTP contra `api.github.com`.
//!
//! Tools registradas en PR2c:
//! - `github.get_repo`   · GET /repos/{owner}/{repo}
//! - `github.list_pulls` · GET /repos/{owner}/{repo}/pulls?state=
//!
//! El token (PAT o GitHub App) se carga desde Vault `github/api_token`
//! (campo `token`). Bearer auth + `X-GitHub-Api-Version` pinneada.

use async_trait::async_trait;
use reqwest::Client;
use serde_json::Value;
use tracing::instrument;

use super::{RegistryBuilder, ToolContext, ToolDef, ToolHandler};
use crate::error::{Error, Result};

const VAULT_PATH: &str = "github/api_token";
const TOKEN_FIELD: &str = "token";
const API_VERSION: &str = "2022-11-28";
const ACCEPT: &str = "application/vnd.github+json";

pub fn register(b: &mut RegistryBuilder, base_url: &str) {
    let http = Client::builder()
        .user_agent(concat!("trenchpass/", env!("CARGO_PKG_VERSION")))
        .build()
        .expect("reqwest client");

    b.register(
        ToolDef {
            id: "github.get_repo".into(),
            namespace: "github",
            description: "Metadata de repositorio (GET /repos/{owner}/{repo}).",
        },
        GetRepo {
            http: http.clone(),
            base_url: base_url.to_string(),
        },
    );

    b.register(
        ToolDef {
            id: "github.list_pulls".into(),
            namespace: "github",
            description: "Lista pull requests del repo (GET /repos/{owner}/{repo}/pulls).",
        },
        ListPulls {
            http,
            base_url: base_url.to_string(),
        },
    );
}

struct GetRepo {
    http: Client,
    base_url: String,
}

struct ListPulls {
    http: Client,
    base_url: String,
}

async fn load_token(ctx: &ToolContext<'_>) -> Result<String> {
    let secret = ctx.vault.secret(VAULT_PATH).await?;
    secret
        .data
        .get("data")
        .and_then(|d| d.get(TOKEN_FIELD))
        .or_else(|| secret.data.get(TOKEN_FIELD))
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| Error::Vault(format!("campo `{TOKEN_FIELD}` ausente en {VAULT_PATH}")))
}

/// Slug GitHub: letras, dígitos, guiones, puntos, underscore. Bloquea inyección de path.
fn valid_slug(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 100
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '.' | '_'))
}

fn extract_owner_repo(params: &Value) -> Result<(&str, &str)> {
    let owner = params
        .get("owner")
        .and_then(Value::as_str)
        .ok_or_else(|| Error::Upstream("github: requiere `owner` (string)".into()))?;
    let repo = params
        .get("repo")
        .and_then(Value::as_str)
        .ok_or_else(|| Error::Upstream("github: requiere `repo` (string)".into()))?;
    if !valid_slug(owner) || !valid_slug(repo) {
        return Err(Error::Upstream(
            "github: owner/repo con caracteres inválidos".into(),
        ));
    }
    Ok((owner, repo))
}

fn map_status(status: reqwest::StatusCode, body: &str) -> Error {
    Error::Upstream(format!("github {status}: {body}"))
}

#[async_trait]
impl ToolHandler for GetRepo {
    #[instrument(skip(self, ctx, params), fields(tool = "github.get_repo"))]
    async fn invoke(&self, ctx: &ToolContext<'_>, params: Value) -> Result<Value> {
        let (owner, repo) = extract_owner_repo(&params)?;
        let token = load_token(ctx).await?;
        let url = format!(
            "{}/repos/{owner}/{repo}",
            self.base_url.trim_end_matches('/')
        );

        let resp = self
            .http
            .get(url)
            .bearer_auth(token)
            .header("Accept", ACCEPT)
            .header("X-GitHub-Api-Version", API_VERSION)
            .send()
            .await
            .map_err(|e| Error::Upstream(format!("github.get_repo send: {e}")))?;

        let status = resp.status();
        let body = resp
            .text()
            .await
            .map_err(|e| Error::Upstream(format!("github.get_repo read: {e}")))?;

        if !status.is_success() {
            return Err(map_status(status, &body));
        }
        serde_json::from_str(&body)
            .map_err(|e| Error::Upstream(format!("github.get_repo parse: {e}")))
    }
}

#[async_trait]
impl ToolHandler for ListPulls {
    #[instrument(skip(self, ctx, params), fields(tool = "github.list_pulls"))]
    async fn invoke(&self, ctx: &ToolContext<'_>, params: Value) -> Result<Value> {
        let (owner, repo) = extract_owner_repo(&params)?;
        let state = params
            .get("state")
            .and_then(Value::as_str)
            .unwrap_or("open");
        if !matches!(state, "open" | "closed" | "all") {
            return Err(Error::Upstream(
                "github.list_pulls: state debe ser open|closed|all".into(),
            ));
        }
        let per_page = params
            .get("per_page")
            .and_then(Value::as_u64)
            .unwrap_or(30)
            .clamp(1, 100);

        let token = load_token(ctx).await?;
        let url = format!(
            "{}/repos/{owner}/{repo}/pulls",
            self.base_url.trim_end_matches('/')
        );

        let resp = self
            .http
            .get(url)
            .bearer_auth(token)
            .header("Accept", ACCEPT)
            .header("X-GitHub-Api-Version", API_VERSION)
            .query(&[
                ("state", state.to_string()),
                ("per_page", per_page.to_string()),
            ])
            .send()
            .await
            .map_err(|e| Error::Upstream(format!("github.list_pulls send: {e}")))?;

        let status = resp.status();
        let body = resp
            .text()
            .await
            .map_err(|e| Error::Upstream(format!("github.list_pulls read: {e}")))?;

        if !status.is_success() {
            return Err(map_status(status, &body));
        }
        serde_json::from_str(&body)
            .map_err(|e| Error::Upstream(format!("github.list_pulls parse: {e}")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn slug_validator() {
        assert!(valid_slug("Alexendros"));
        assert!(valid_slug("trenchpass"));
        assert!(valid_slug("rust-lang"));
        assert!(valid_slug("my.repo_v2"));
        assert!(!valid_slug(""));
        assert!(!valid_slug("../etc"));
        assert!(!valid_slug("owner/sneak"));
        assert!(!valid_slug(&"a".repeat(101)));
    }

    #[tokio::test]
    async fn get_repo_sends_bearer_and_api_version() {
        use wiremock::matchers::{header, method, path};
        use wiremock::{Mock, MockServer, ResponseTemplate};

        let server = MockServer::start().await;
        let body = json!({"id": 1, "full_name": "Alexendros/trenchpass"});
        Mock::given(method("GET"))
            .and(path("/repos/Alexendros/trenchpass"))
            .and(header("Authorization", "Bearer ghp_xxx"))
            .and(header("X-GitHub-Api-Version", API_VERSION))
            .and(header("Accept", ACCEPT))
            .respond_with(ResponseTemplate::new(200).set_body_json(body.clone()))
            .expect(1)
            .mount(&server)
            .await;

        let mut b = RegistryBuilder::default();
        register(&mut b, &server.uri());
        let registry = b.finish();
        let (_, handler) = registry.get("github.get_repo").expect("registered");

        let vault = crate::vault::VaultClient::test_with_secret(
            VAULT_PATH,
            json!({"data": {"token": "ghp_xxx"}}),
        );
        let consumer = crate::auth::Consumer::dev("test");
        let ctx = ToolContext {
            consumer: &consumer,
            vault: &vault,
        };

        let out = handler
            .invoke(&ctx, json!({"owner": "Alexendros", "repo": "trenchpass"}))
            .await
            .expect("ok");
        assert_eq!(out, body);
    }

    #[tokio::test]
    async fn list_pulls_rejects_invalid_state() {
        let mut b = RegistryBuilder::default();
        register(&mut b, "http://example.invalid");
        let registry = b.finish();
        let (_, handler) = registry.get("github.list_pulls").expect("registered");

        let vault = crate::vault::VaultClient::test_with_secret(
            VAULT_PATH,
            json!({"data": {"token": "ghp_xxx"}}),
        );
        let consumer = crate::auth::Consumer::dev("test");
        let ctx = ToolContext {
            consumer: &consumer,
            vault: &vault,
        };

        let err = handler
            .invoke(
                &ctx,
                json!({"owner": "Alexendros", "repo": "trenchpass", "state": "bogus"}),
            )
            .await
            .expect_err("estado inválido");
        assert!(err.to_string().contains("state"));
    }
}
