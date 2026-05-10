//! Registry de tools por namespace.
//!
//! Cada namespace expone una lista de `ToolDef` (id, descripción, schema) y un
//! `dispatch(consumer, params)`. PR2 implementa Notion, Stripe, GitHub. PR5 el resto.

use std::collections::BTreeMap;
use std::sync::Arc;

use serde::Serialize;

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

#[derive(Debug, Clone, Serialize)]
pub struct ToolDef {
    pub id: String,
    pub namespace: &'static str,
    pub description: &'static str,
}

#[derive(Default)]
pub struct ToolRegistry {
    by_id: BTreeMap<String, ToolDef>,
}

impl ToolRegistry {
    pub fn build() -> Arc<Self> {
        let mut by_id = BTreeMap::new();
        for ns in NAMESPACES {
            for tool in (ns.tools)() {
                by_id.insert(tool.id.clone(), tool);
            }
        }
        Arc::new(Self { by_id })
    }

    pub fn list(&self) -> Vec<&ToolDef> {
        self.by_id.values().collect()
    }

    pub fn namespaces(&self) -> Vec<&'static str> {
        NAMESPACES.iter().map(|n| n.name).collect()
    }
}

pub struct Namespace {
    pub name: &'static str,
    pub tools: fn() -> Vec<ToolDef>,
}

const NAMESPACES: &[Namespace] = &[
    Namespace {
        name: "notion",
        tools: notion::tools,
    },
    Namespace {
        name: "stripe",
        tools: stripe::tools,
    },
    Namespace {
        name: "github",
        tools: github::tools,
    },
    Namespace {
        name: "forgejo",
        tools: forgejo::tools,
    },
    Namespace {
        name: "dokploy",
        tools: dokploy::tools,
    },
    Namespace {
        name: "hostinger",
        tools: hostinger::tools,
    },
    Namespace {
        name: "vercel",
        tools: vercel::tools,
    },
    Namespace {
        name: "n8n",
        tools: n8n::tools,
    },
    Namespace {
        name: "glitchtip",
        tools: glitchtip::tools,
    },
    Namespace {
        name: "docuseal",
        tools: docuseal::tools,
    },
    Namespace {
        name: "proton",
        tools: proton::tools,
    },
    Namespace {
        name: "gocardless_dd",
        tools: gocardless_dd::tools,
    },
    Namespace {
        name: "gocardless_psd2",
        tools: gocardless_psd2::tools,
    },
];
