//! Namespace `forgejo.*` · cliente HTTP contra una instancia Forgejo
//! self-hosted (`api/v1/repos/search`).

use async_trait::async_trait;
use serde_json::Value;
use tracing::instrument;

use super::shared::{bearer_get_json, http_client, load_secret_field};
use super::{RegistryBuilder, ToolContext, ToolDef, ToolHandler};
use crate::error::{Error, Result};

const VAULT_PATH: &str = "forgejo/api_token";
const TOKEN_FIELD: &str = "token";
const NS: &str = "forgejo";

pub fn register(b: &mut RegistryBuilder, base_url: &str) {
    let http = http_client();
    b.register(
        ToolDef {
            id: "forgejo.search_repos".into(),
            namespace: NS,
            description: "Busca repositorios en una instancia Forgejo (GET /api/v1/repos/search).",
        },
        SearchRepos {
            http,
            base_url: base_url.to_string(),
        },
    );
}

struct SearchRepos {
    http: reqwest::Client,
    base_url: String,
}

#[async_trait]
impl ToolHandler for SearchRepos {
    #[instrument(skip(self, ctx, params), fields(tool = "forgejo.search_repos"))]
    async fn invoke(&self, ctx: &ToolContext<'_>, params: Value) -> Result<Value> {
        let q = params
            .get("q")
            .and_then(Value::as_str)
            .ok_or_else(|| Error::Upstream("forgejo.search_repos requiere `q` (string)".into()))?;
        let limit = params
            .get("limit")
            .and_then(Value::as_u64)
            .unwrap_or(20)
            .min(50);
        let token = load_secret_field(ctx, VAULT_PATH, TOKEN_FIELD).await?;
        let url = format!(
            "{}/api/v1/repos/search?q={}&limit={}",
            self.base_url.trim_end_matches('/'),
            urlencode_q(q),
            limit
        );
        bearer_get_json(&self.http, NS, &url, &token, &[]).await
    }
}

fn urlencode_q(s: &str) -> String {
    percent_encoding::utf8_percent_encode(s, percent_encoding::NON_ALPHANUMERIC).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn urlencode_basico() {
        assert_eq!(urlencode_q("hola mundo"), "hola%20mundo");
        assert_eq!(urlencode_q("a&b"), "a%26b");
    }
}
