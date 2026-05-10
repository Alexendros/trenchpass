//! Namespace `dokploy.*` · cliente HTTP contra una instancia Dokploy
//! self-hosted (`/api/projects.list`).

use async_trait::async_trait;
use serde_json::Value;
use tracing::instrument;

use super::shared::{bearer_get_json, http_client, load_secret_field};
use super::{RegistryBuilder, ToolContext, ToolDef, ToolHandler};
use crate::error::Result;

const VAULT_PATH: &str = "dokploy/api_token";
const TOKEN_FIELD: &str = "token";
const NS: &str = "dokploy";

pub fn register(b: &mut RegistryBuilder, base_url: &str) {
    let http = http_client();
    b.register(
        ToolDef {
            id: "dokploy.list_projects".into(),
            namespace: NS,
            description: "Lista los proyectos en Dokploy (GET /api/projects.list).",
        },
        ListProjects {
            http,
            base_url: base_url.to_string(),
        },
    );
}

struct ListProjects {
    http: reqwest::Client,
    base_url: String,
}

#[async_trait]
impl ToolHandler for ListProjects {
    #[instrument(skip(self, ctx, _params), fields(tool = "dokploy.list_projects"))]
    async fn invoke(&self, ctx: &ToolContext<'_>, _params: Value) -> Result<Value> {
        let token = load_secret_field(ctx, VAULT_PATH, TOKEN_FIELD).await?;
        let url = format!("{}/api/projects.list", self.base_url.trim_end_matches('/'));
        bearer_get_json(&self.http, NS, &url, &token, &[]).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_anade_list_projects() {
        let mut b = RegistryBuilder::default();
        register(&mut b, "http://example.invalid");
        let reg = b.finish();
        assert!(reg.get("dokploy.list_projects").is_some());
    }
}
