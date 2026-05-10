//! Namespace `vercel.*` · API REST Vercel (`/v9/projects`).

use async_trait::async_trait;
use serde_json::Value;
use tracing::instrument;

use super::shared::{bearer_get_json, http_client, load_secret_field};
use super::{RegistryBuilder, ToolContext, ToolDef, ToolHandler};
use crate::error::Result;

const VAULT_PATH: &str = "vercel/api_token";
const TOKEN_FIELD: &str = "token";
const NS: &str = "vercel";

pub fn register(b: &mut RegistryBuilder, base_url: &str) {
    let http = http_client();
    b.register(
        ToolDef {
            id: "vercel.list_projects".into(),
            namespace: NS,
            description: "Lista proyectos del equipo Vercel (GET /v9/projects).",
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
    #[instrument(skip(self, ctx, params), fields(tool = "vercel.list_projects"))]
    async fn invoke(&self, ctx: &ToolContext<'_>, params: Value) -> Result<Value> {
        let limit = params
            .get("limit")
            .and_then(Value::as_u64)
            .unwrap_or(20)
            .min(100);
        let token = load_secret_field(ctx, VAULT_PATH, TOKEN_FIELD).await?;
        let url = format!(
            "{}/v9/projects?limit={limit}",
            self.base_url.trim_end_matches('/')
        );
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
        assert!(reg.get("vercel.list_projects").is_some());
    }
}
