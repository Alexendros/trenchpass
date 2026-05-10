//! Namespace `docuseal.*` · API DocuSeal self-hosted.
//! DocuSeal usa header `X-Auth-Token`.

use async_trait::async_trait;
use reqwest::Client;
use serde_json::Value;
use tracing::instrument;

use super::shared::{auth_get_json, http_client, load_secret_field, AuthScheme};
use super::{RegistryBuilder, ToolContext, ToolDef, ToolHandler};
use crate::error::Result;

const VAULT_PATH: &str = "docuseal/api_token";
const TOKEN_FIELD: &str = "token";
const NS: &str = "docuseal";

pub fn register(b: &mut RegistryBuilder, base_url: &str) {
    let http = http_client();
    b.register(
        ToolDef {
            id: "docuseal.list_templates".into(),
            namespace: NS,
            description: "Lista plantillas de documentos DocuSeal (GET /templates).",
        },
        ListTemplates {
            http,
            base_url: base_url.to_string(),
        },
    );
}

struct ListTemplates {
    http: Client,
    base_url: String,
}

#[async_trait]
impl ToolHandler for ListTemplates {
    #[instrument(skip(self, ctx, _params), fields(tool = "docuseal.list_templates"))]
    async fn invoke(&self, ctx: &ToolContext<'_>, _params: Value) -> Result<Value> {
        let token = load_secret_field(ctx, VAULT_PATH, TOKEN_FIELD).await?;
        let url = format!("{}/templates", self.base_url.trim_end_matches('/'));
        auth_get_json(
            &self.http,
            NS,
            &url,
            &AuthScheme::Header {
                name: "X-Auth-Token",
                value: token,
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
    fn register_anade_list_templates() {
        let mut b = RegistryBuilder::default();
        register(&mut b, "http://example.invalid");
        let reg = b.finish();
        assert!(reg.get("docuseal.list_templates").is_some());
    }
}
