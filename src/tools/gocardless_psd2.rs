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
        if !is_valid_account_id(id) {
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

/// Predicate compartido entre handler y tests · evita drift de validación.
/// UUID-like (GoCardless emite IDs alfanuméricos con guiones · max 64 chars).
fn is_valid_account_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 64 && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_anade_get_account() {
        let mut b = RegistryBuilder::default();
        register(&mut b, "http://example.invalid");
        let reg = b.finish();
        assert!(reg.get("gocardless_psd2.get_account").is_some());
    }

    #[test]
    fn is_valid_account_id_acepta_uuid_y_alfanumerico() {
        assert!(is_valid_account_id("3fa85f64-5717-4562-b3fc-2c963f66afa6"));
        assert!(is_valid_account_id("ABC123"));
        assert!(is_valid_account_id(&"a".repeat(64)));
    }

    #[test]
    fn is_valid_account_id_rechaza_empty_largo_y_caracteres_invalidos() {
        assert!(!is_valid_account_id(""));
        assert!(!is_valid_account_id(&"a".repeat(65)));
        assert!(!is_valid_account_id("../etc/passwd"));
        assert!(!is_valid_account_id("acc/foo"));
        assert!(!is_valid_account_id("acc@evil"));
    }
}
