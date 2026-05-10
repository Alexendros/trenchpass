//! Namespace `glitchtip.*` · API GlitchTip (compatibility con Sentry,
//! `/api/0/organizations/{slug}/projects/`).

use async_trait::async_trait;
use serde_json::Value;
use tracing::instrument;

use super::shared::{bearer_get_json, http_client, load_secret_field};
use super::{RegistryBuilder, ToolContext, ToolDef, ToolHandler};
use crate::error::{Error, Result};

const VAULT_PATH: &str = "glitchtip/api_token";
const TOKEN_FIELD: &str = "token";
const NS: &str = "glitchtip";

pub fn register(b: &mut RegistryBuilder, base_url: &str) {
    let http = http_client();
    b.register(
        ToolDef {
            id: "glitchtip.list_projects".into(),
            namespace: NS,
            description: "Lista proyectos en una organización GlitchTip.",
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
    #[instrument(skip(self, ctx, params), fields(tool = "glitchtip.list_projects"))]
    async fn invoke(&self, ctx: &ToolContext<'_>, params: Value) -> Result<Value> {
        let org = params
            .get("org_slug")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                Error::Upstream("glitchtip.list_projects requiere `org_slug` (string)".into())
            })?;
        if !is_valid_org_slug(org) {
            return Err(Error::Upstream("glitchtip: org_slug inválido".into()));
        }
        let token = load_secret_field(ctx, VAULT_PATH, TOKEN_FIELD).await?;
        let url = format!(
            "{}/api/0/organizations/{org}/projects/",
            self.base_url.trim_end_matches('/')
        );
        bearer_get_json(&self.http, NS, &url, &token, &[]).await
    }
}

/// Predicate compartido entre handler y tests · evita drift de validación.
/// Acepta `[A-Za-z0-9_-]{1,100}` no vacío.
fn is_valid_org_slug(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 100
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_anade_list_projects() {
        let mut b = RegistryBuilder::default();
        register(&mut b, "http://example.invalid");
        let reg = b.finish();
        assert!(reg.get("glitchtip.list_projects").is_some());
    }

    #[test]
    fn is_valid_org_slug_acepta_alfanumerico_guion_underscore() {
        assert!(is_valid_org_slug("acme"));
        assert!(is_valid_org_slug("acme-corp"));
        assert!(is_valid_org_slug("acme_corp_2"));
        assert!(is_valid_org_slug(&"x".repeat(100)));
    }

    #[test]
    fn is_valid_org_slug_rechaza_empty_y_largo_y_caracteres_invalidos() {
        assert!(!is_valid_org_slug(""));
        assert!(!is_valid_org_slug(&"a".repeat(101)));
        assert!(!is_valid_org_slug("acme/evil"));
        assert!(!is_valid_org_slug("acme.evil"));
        assert!(!is_valid_org_slug("acme evil"));
    }
}
