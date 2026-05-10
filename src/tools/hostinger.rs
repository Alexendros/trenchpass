//! Namespace `hostinger.*` · API Hostinger v1 (`/api/vps/v1/virtual-machines`).

use async_trait::async_trait;
use serde_json::Value;
use tracing::instrument;

use super::shared::{bearer_get_json, http_client, load_secret_field};
use super::{RegistryBuilder, ToolContext, ToolDef, ToolHandler};
use crate::error::Result;

const VAULT_PATH: &str = "hostinger/api_token";
const TOKEN_FIELD: &str = "token";
const NS: &str = "hostinger";

pub fn register(b: &mut RegistryBuilder, base_url: &str) {
    let http = http_client();
    b.register(
        ToolDef {
            id: "hostinger.list_vps".into(),
            namespace: NS,
            description: "Lista las VPS del operador (GET /api/vps/v1/virtual-machines).",
        },
        ListVps {
            http,
            base_url: base_url.to_string(),
        },
    );
}

struct ListVps {
    http: reqwest::Client,
    base_url: String,
}

#[async_trait]
impl ToolHandler for ListVps {
    #[instrument(skip(self, ctx, _params), fields(tool = "hostinger.list_vps"))]
    async fn invoke(&self, ctx: &ToolContext<'_>, _params: Value) -> Result<Value> {
        let token = load_secret_field(ctx, VAULT_PATH, TOKEN_FIELD).await?;
        let url = format!(
            "{}/api/vps/v1/virtual-machines",
            self.base_url.trim_end_matches('/')
        );
        bearer_get_json(&self.http, NS, &url, &token, &[]).await
    }
}
