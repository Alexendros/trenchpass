//! Namespace `n8n.*` · API n8n self-hosted (`/api/v1/workflows`).
//! n8n usa header `X-N8N-API-KEY` en lugar de Bearer.

use async_trait::async_trait;
use reqwest::Client;
use serde_json::Value;
use tracing::instrument;

use super::shared::{auth_get_json, http_client, load_secret_field, AuthScheme};
use super::{RegistryBuilder, ToolContext, ToolDef, ToolHandler};
use crate::error::Result;

const VAULT_PATH: &str = "n8n/api_key";
const TOKEN_FIELD: &str = "key";
const NS: &str = "n8n";

pub fn register(b: &mut RegistryBuilder, base_url: &str) {
    let http = http_client();
    b.register(
        ToolDef {
            id: "n8n.list_workflows".into(),
            namespace: NS,
            description: "Lista workflows de n8n (GET /api/v1/workflows).",
        },
        ListWorkflows {
            http,
            base_url: base_url.to_string(),
        },
    );
}

struct ListWorkflows {
    http: Client,
    base_url: String,
}

#[async_trait]
impl ToolHandler for ListWorkflows {
    #[instrument(skip(self, ctx, _params), fields(tool = "n8n.list_workflows"))]
    async fn invoke(&self, ctx: &ToolContext<'_>, _params: Value) -> Result<Value> {
        let key = load_secret_field(ctx, VAULT_PATH, TOKEN_FIELD).await?;
        let url = format!("{}/api/v1/workflows", self.base_url.trim_end_matches('/'));
        auth_get_json(
            &self.http,
            NS,
            &url,
            &AuthScheme::Header {
                name: "X-N8N-API-KEY",
                value: key,
            },
            &[],
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_anade_list_workflows() {
        let mut b = RegistryBuilder::default();
        register(&mut b, "http://example.invalid");
        let reg = b.finish();
        assert!(reg.get("n8n.list_workflows").is_some());
    }
}
