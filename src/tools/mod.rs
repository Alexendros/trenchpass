//! Registry de tools por namespace + trait de dispatch.
//!
//! Modelo:
//! - Cada namespace expone una función `register(&mut RegistryBuilder, ctx)` que
//!   inserta sus tools (id estable + handler async).
//! - El handler implementa [`ToolHandler`], recibe un [`ToolContext`] efímero y
//!   devuelve `serde_json::Value`.
//! - [`dispatch`] ata: lookup → scope check → invoke → audit best-effort.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::time::Instant;

use async_trait::async_trait;
use serde::Serialize;
use serde_json::Value;

use crate::audit::{AuditEvent, AuditOutcome, AuditStore};
use crate::auth::{scope, Consumer};
use crate::error::{Error, Result};
use crate::vault::VaultClient;

pub mod docuseal;
pub mod dokploy;
pub mod forgejo;
pub mod github;
pub mod glitchtip;
pub mod gocardless_dd;
pub mod gocardless_psd2;
pub mod hostinger;
pub mod n8n;
pub mod notion;
pub mod proton;
mod shared;
pub mod stripe;
pub mod vercel;

/// Metadata pública (expuesta en `GET /tools`).
#[derive(Debug, Clone, Serialize)]
pub struct ToolDef {
    pub id: String,
    pub namespace: &'static str,
    pub description: &'static str,
}

/// Trait de dispatch async. Stateless en sí mismo: el estado vive en el handler.
#[async_trait]
pub trait ToolHandler: Send + Sync {
    async fn invoke(&self, ctx: &ToolContext<'_>, params: Value) -> Result<Value>;
}

/// Contexto efímero por request — no se almacena.
pub struct ToolContext<'a> {
    pub consumer: &'a Consumer,
    pub vault: &'a VaultClient,
}

struct ToolEntry {
    def: ToolDef,
    handler: Arc<dyn ToolHandler>,
}

#[derive(Default)]
pub struct RegistryBuilder {
    by_id: BTreeMap<String, ToolEntry>,
}

impl RegistryBuilder {
    pub fn register<H: ToolHandler + 'static>(&mut self, def: ToolDef, handler: H) -> &mut Self {
        self.by_id.insert(
            def.id.clone(),
            ToolEntry {
                def,
                handler: Arc::new(handler),
            },
        );
        self
    }

    fn finish(self) -> ToolRegistry {
        ToolRegistry { by_id: self.by_id }
    }
}

pub struct ToolRegistry {
    by_id: BTreeMap<String, ToolEntry>,
}

/// Base URLs por namespace · permite override por test (wiremock) y por
/// configuración (instancias self-hosted Forgejo / Dokploy / GlitchTip /
/// DocuSeal / n8n).
#[derive(Debug, Clone)]
pub struct BaseUrls {
    pub notion: String,
    pub stripe: String,
    pub github: String,
    pub forgejo: String,
    pub dokploy: String,
    pub hostinger: String,
    pub vercel: String,
    pub n8n: String,
    pub glitchtip: String,
    pub docuseal: String,
    pub gocardless_dd: String,
    pub gocardless_psd2: String,
}

impl BaseUrls {
    pub fn production() -> Self {
        Self {
            notion: "https://api.notion.com/v1".into(),
            stripe: "https://api.stripe.com/v1".into(),
            github: "https://api.github.com".into(),
            // Self-hosted: el operador debe configurar URL real vía env
            // (ver `BaseUrls::from_env`). Defaults se sobrescriben en prod.
            forgejo: "https://forgejo.local".into(),
            dokploy: "https://dokploy.local".into(),
            hostinger: "https://developers.hostinger.com".into(),
            vercel: "https://api.vercel.com".into(),
            n8n: "https://n8n.local".into(),
            glitchtip: "https://glitchtip.local".into(),
            docuseal: "https://docuseal.local".into(),
            gocardless_dd: "https://api.gocardless.com".into(),
            gocardless_psd2: "https://bankaccountdata.gocardless.com".into(),
        }
    }
}

impl ToolRegistry {
    /// Construye el registry con upstreams en producción.
    pub fn build() -> Arc<Self> {
        Self::with_bases(BaseUrls::production())
    }

    /// Variante mantenida para tests existentes (solo notion).
    pub fn with_notion_base(notion_base: &str) -> Arc<Self> {
        Self::with_bases(BaseUrls {
            notion: notion_base.into(),
            ..BaseUrls::production()
        })
    }

    /// Construye el registry permitiendo override de TODOS los base URLs.
    /// Usado por tests para apuntar a wiremock.
    pub fn with_bases(b: BaseUrls) -> Arc<Self> {
        let mut rb = RegistryBuilder::default();
        notion::register(&mut rb, &b.notion);
        stripe::register(&mut rb, &b.stripe);
        github::register(&mut rb, &b.github);
        forgejo::register(&mut rb, &b.forgejo);
        dokploy::register(&mut rb, &b.dokploy);
        hostinger::register(&mut rb, &b.hostinger);
        vercel::register(&mut rb, &b.vercel);
        n8n::register(&mut rb, &b.n8n);
        glitchtip::register(&mut rb, &b.glitchtip);
        docuseal::register(&mut rb, &b.docuseal);
        gocardless_dd::register(&mut rb, &b.gocardless_dd);
        gocardless_psd2::register(&mut rb, &b.gocardless_psd2);
        proton::register(&mut rb);
        Arc::new(rb.finish())
    }

    pub fn list(&self) -> Vec<&ToolDef> {
        self.by_id.values().map(|e| &e.def).collect()
    }

    pub fn namespaces(&self) -> Vec<&'static str> {
        let mut ns: Vec<&'static str> = self.by_id.values().map(|e| e.def.namespace).collect();
        ns.sort_unstable();
        ns.dedup();
        ns
    }

    pub fn get(&self, id: &str) -> Option<(&ToolDef, Arc<dyn ToolHandler>)> {
        self.by_id.get(id).map(|e| (&e.def, Arc::clone(&e.handler)))
    }
}

/// Punto de entrada único para el endpoint HTTP. Hace scope check, invoca y audita.
///
/// El audit es **best-effort** (no bloquea respuesta si Postgres falla).
pub async fn dispatch(
    registry: &ToolRegistry,
    audit: &AuditStore,
    vault: &VaultClient,
    consumer: &Consumer,
    tool_id: &str,
    params: Value,
) -> Result<Value> {
    let started = Instant::now();

    let (def, handler) = registry
        .get(tool_id)
        .ok_or_else(|| Error::NotFound(format!("tool {tool_id}")))?;

    if let Err(e) = scope::check(&def.id, &consumer.scopes) {
        audit
            .record_best_effort(AuditEvent {
                consumer_id: &consumer.id,
                action: "tool_call",
                namespace: def.namespace,
                secret_path: tool_id,
                outcome: AuditOutcome::Denied,
                latency_ms: Some(elapsed_ms(started)),
                detail: Some(serde_json::json!({"error": "scope_violation"})),
            })
            .await;
        return Err(e);
    }

    let ctx = ToolContext { consumer, vault };
    let result = handler.invoke(&ctx, params).await;

    let outcome = match &result {
        Ok(_) => AuditOutcome::Ok,
        Err(_) => AuditOutcome::Error,
    };
    audit
        .record_best_effort(AuditEvent {
            consumer_id: &consumer.id,
            action: "tool_call",
            namespace: def.namespace,
            secret_path: tool_id,
            outcome,
            latency_ms: Some(elapsed_ms(started)),
            detail: result
                .as_ref()
                .err()
                .map(|e| serde_json::json!({"error": e.to_string()})),
        })
        .await;

    result
}

fn elapsed_ms(t0: Instant) -> i32 {
    t0.elapsed().as_millis().min(i32::MAX as u128) as i32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_lists_notion_namespace() {
        let reg = ToolRegistry::with_notion_base("http://example.invalid");
        let ns = reg.namespaces();
        assert!(ns.contains(&"notion"), "namespaces: {ns:?}");
        let ids: Vec<_> = reg.list().iter().map(|d| d.id.as_str()).collect();
        assert!(ids.contains(&"notion.search"), "ids: {ids:?}");
        assert!(ids.contains(&"notion.fetch"), "ids: {ids:?}");
    }

    /// PR5: los 13 namespaces deben estar registrados en el registry default.
    #[test]
    fn registry_cubre_los_13_namespaces() {
        let reg = ToolRegistry::with_bases(BaseUrls::production());
        let ns = reg.namespaces();
        let expected = [
            "notion",
            "stripe",
            "github",
            "forgejo",
            "dokploy",
            "hostinger",
            "vercel",
            "n8n",
            "glitchtip",
            "docuseal",
            "gocardless_dd",
            "gocardless_psd2",
            "proton",
        ];
        for n in expected {
            assert!(
                ns.contains(&n),
                "namespace `{n}` ausente · presentes: {ns:?}"
            );
        }
        assert_eq!(ns.len(), expected.len(), "duplicados o sobrantes: {ns:?}");
    }
}
