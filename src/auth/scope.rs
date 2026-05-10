//! Validación de scopes: `notion:*`, `stripe:read`, `*`...
//!
//! Reglas:
//! - `*` cubre cualquier tool.
//! - `<namespace>:*` cubre todas las acciones de ese namespace.
//! - `<namespace>:<verb>_<resource>` exact match contra `tool` (ej. `stripe.list_subscriptions`).
//!   La función traduce `notion.search_pages` → `notion:search_pages` antes de comparar.

use crate::error::{Error, Result};

/// Convierte el id de tool MCP (`namespace.verb_resource`) en formato scope (`namespace:verb_resource`).
pub fn tool_to_scope(tool: &str) -> String {
    tool.replacen('.', ":", 1)
}

pub fn check(tool: &str, granted: &[String]) -> Result<()> {
    let needed = tool_to_scope(tool);
    let (namespace, _verb) = needed.split_once(':').unwrap_or((needed.as_str(), ""));

    let allowed = granted.iter().any(|g| {
        g == "*"
            || g == &needed
            || g.strip_suffix(":*")
                .map(|ns| ns == namespace)
                .unwrap_or(false)
    });

    if allowed {
        Ok(())
    } else {
        Err(Error::ScopeViolation {
            required: needed,
            granted: granted.to_vec(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wildcard_total() {
        check("notion.search_pages", &["*".into()]).unwrap();
    }

    #[test]
    fn wildcard_namespace() {
        check("stripe.list_subscriptions", &["stripe:*".into()]).unwrap();
    }

    #[test]
    fn exact_match() {
        check("github.list_prs", &["github:list_prs".into()]).unwrap();
    }

    #[test]
    fn rejects_other_namespace() {
        let err = check("stripe.cancel_subscription", &["notion:*".into()]).unwrap_err();
        assert!(matches!(err, Error::ScopeViolation { .. }));
    }
}
