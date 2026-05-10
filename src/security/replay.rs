//! Replay protection: nonce + timestamp por request firmado.
//!
//! El consumidor envía `X-TrenchPass-Nonce: <uuid>` y `X-TrenchPass-Timestamp: <unix>`.
//! Reglas:
//! - timestamp dentro de ±5 min del wall-clock.
//! - nonce no visto en los últimos 5 min.

use std::sync::Arc;
use std::time::{Duration, Instant};

use dashmap::DashMap;

const WINDOW: Duration = Duration::from_secs(300);
const SKEW: i64 = 300;

#[derive(Clone)]
pub struct ReplayCache {
    seen: Arc<DashMap<String, Instant>>,
}

impl ReplayCache {
    pub fn new() -> Self {
        Self {
            seen: Arc::new(DashMap::new()),
        }
    }

    /// Devuelve `true` si el `(nonce, timestamp)` es nuevo y está dentro de skew.
    /// `false` si es replay o timestamp fuera de la ventana.
    /// El middleware traduce `false` a `Error::Replay`.
    pub fn check(&self, nonce: &str, timestamp: i64, now_unix: i64) -> bool {
        if (now_unix - timestamp).abs() > SKEW {
            return false;
        }
        // GC oportunista: ~1 de cada 64 inserts purgamos entradas vencidas.
        if !self.seen.is_empty() && nonce.as_bytes().first().copied().unwrap_or(0) % 64 == 0 {
            self.gc();
        }
        match self.seen.entry(nonce.to_string()) {
            dashmap::mapref::entry::Entry::Occupied(_) => false,
            dashmap::mapref::entry::Entry::Vacant(slot) => {
                slot.insert(Instant::now());
                true
            }
        }
    }

    fn gc(&self) {
        let cutoff = Instant::now() - WINDOW;
        self.seen.retain(|_, ts| *ts >= cutoff);
    }
}

impl Default for ReplayCache {
    fn default() -> Self {
        Self::new()
    }
}
