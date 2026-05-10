//! Namespace `gocardless_dd.*` · GoCardless Direct Debit / Pay (`api.gocardless.com`).

use async_trait::async_trait;
use serde_json::Value;
use tracing::instrument;

use super::shared::{bearer_get_json, http_client, load_secret_field};
use super::{RegistryBuilder, ToolContext, ToolDef, ToolHandler};
use crate::error::Result;

const VAULT_PATH: &str = "gocardless_dd/api_token";
const TOKEN_FIELD: &str = "token";
const NS: &str = "gocardless_dd";
/// Header documentado por GoCardless (versión SemVer fija para reproducibilidad).
const GC_VERSION: &str = "2015-07-06";

pub fn register(b: &mut RegistryBuilder, base_url: &str) {
    let http = http_client();
    b.register(
        ToolDef {
            id: "gocardless_dd.list_customers".into(),
            namespace: NS,
            description: "Lista clientes Direct Debit (GET /customers).",
        },
        ListCustomers {
            http,
            base_url: base_url.to_string(),
        },
    );
}

struct ListCustomers {
    http: reqwest::Client,
    base_url: String,
}

#[async_trait]
impl ToolHandler for ListCustomers {
    #[instrument(skip(self, ctx, params), fields(tool = "gocardless_dd.list_customers"))]
    async fn invoke(&self, ctx: &ToolContext<'_>, params: Value) -> Result<Value> {
        let limit = params
            .get("limit")
            .and_then(Value::as_u64)
            .unwrap_or(50)
            .clamp(1, 500);
        let token = load_secret_field(ctx, VAULT_PATH, TOKEN_FIELD).await?;
        let url = format!(
            "{}/customers?limit={limit}",
            self.base_url.trim_end_matches('/')
        );
        bearer_get_json(
            &self.http,
            NS,
            &url,
            &token,
            &[("GoCardless-Version", GC_VERSION)],
        )
        .await
    }
}
