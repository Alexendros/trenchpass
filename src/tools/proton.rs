//! Namespace `proton.*` · ProtonMail/ProtonDrive no exponen API REST pública
//! para usuarios finales (sólo Proton Bridge IMAP/SMTP local). Esta tool
//! valida que el secret está custodiado en Vault con la forma esperada,
//! sirviendo como placeholder hasta integrar Proton Bridge en PR7 (vía-fax)
//! o el SDK oficial cuando exista.

use async_trait::async_trait;
use serde_json::{json, Value};
use tracing::instrument;

use super::shared::load_secret_field;
use super::{RegistryBuilder, ToolContext, ToolDef, ToolHandler};
use crate::error::Result;

const VAULT_PATH: &str = "proton/credentials";
const FIELD_USER: &str = "username";

pub fn register(b: &mut RegistryBuilder) {
    b.register(
        ToolDef {
            id: "proton.check_credentials".into(),
            namespace: "proton",
            description: "Verifica que las credenciales Proton existen en Vault (no hace login). \
                 Placeholder hasta integración Proton Bridge / SDK oficial.",
        },
        CheckCredentials,
    );
}

struct CheckCredentials;

#[async_trait]
impl ToolHandler for CheckCredentials {
    #[instrument(skip(self, ctx, _params), fields(tool = "proton.check_credentials"))]
    async fn invoke(&self, ctx: &ToolContext<'_>, _params: Value) -> Result<Value> {
        let user = load_secret_field(ctx, VAULT_PATH, FIELD_USER).await?;
        // No exponemos el password ni en logs.
        Ok(json!({
            "vault_path": VAULT_PATH,
            "username_present": true,
            "username_redacted": redact(&user),
            "note": "Proton no tiene REST API pública · placeholder hasta PR7 vía-fax / SDK oficial"
        }))
    }
}

fn redact(s: &str) -> String {
    let n = s.chars().count();
    if n <= 4 {
        return "***".into();
    }
    let head: String = s.chars().take(2).collect();
    let tail: String = s
        .chars()
        .rev()
        .take(2)
        .collect::<String>()
        .chars()
        .rev()
        .collect();
    format!("{head}***{tail}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redact_preserva_borde_oculta_centro() {
        assert_eq!(redact("alex@proton.me"), "al***me");
        assert_eq!(redact("ab"), "***");
    }
}
