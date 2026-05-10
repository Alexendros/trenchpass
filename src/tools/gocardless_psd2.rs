//! Namespace `gocardless_psd2.*` · GoCardless Bank Account Data (PSD2 AIS),
//! `bankaccountdata.gocardless.com/api/v2/`.

use async_trait::async_trait;
use serde_json::Value;
use tracing::instrument;

use super::shared::{bearer_get_json, http_client, load_secret_field};
use super::{RegistryBuilder, ToolContext, ToolDef, ToolHandler};
use crate::error::{Error, Result};

const VAULT_PATH: &str = "gocardless_psd2/access_token";
const TOKEN_FIELD: &str = "access";
const NS: &str = "gocardless_psd2";

pub fn register(b: &mut RegistryBuilder, base_url: &str) {
    let http = http_client();
    b.register(
        ToolDef {
            id: "gocardless_psd2.get_account".into(),
            namespace: NS,
            description: "Lee detalles de una cuenta bancaria PSD2 (GET /api/v2/accounts/{id}/).",
        },
        GetAccount {
            http,
            base_url: base_url.to_string(),
        },
    );
}

struct GetAccount {
    http: reqwest::Client,
    base_url: String,
}

#[async_trait]
impl ToolHandler for GetAccount {
    #[instrument(skip(self, ctx, params), fields(tool = "gocardless_psd2.get_account"))]
    async fn invoke(&self, ctx: &ToolContext<'_>, params: Value) -> Result<Value> {
        let id = params
            .get("account_id")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                Error::Upstream("gocardless_psd2.get_account requiere `account_id`".into())
            })?;
        // UUID-like guard (GoCardless usa UUIDs sin guiones tipo).
        if !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-') || id.len() > 64 {
            return Err(Error::Upstream(
                "gocardless_psd2: account_id inválido".into(),
            ));
        }
        let token = load_secret_field(ctx, VAULT_PATH, TOKEN_FIELD).await?;
        let url = format!(
            "{}/api/v2/accounts/{id}/",
            self.base_url.trim_end_matches('/')
        );
        bearer_get_json(&self.http, NS, &url, &token, &[]).await
    }
}
