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

impl ToolRegistry {
    /// Construye el registry con upstreams en producción.
    pub fn build() -> Arc<Self> {
        Self::with_notion_base("https://api.notion.com/v1")
    }

    /// Construye el registry permitiendo override del base URL de Notion (tests).
    pub fn with_notion_base(notion_base: &str) -> Arc<Self> {
        let mut b = RegistryBuilder::default();
        notion::register(&mut b, notion_base);
        // Resto de namespaces se cablean en PR2c (stripe + github) y PR5.
        Arc::new(b.finish())
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
}
