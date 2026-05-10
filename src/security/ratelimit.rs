//! Rate limit por consumidor (token bucket vía `governor`).
//!
//! Cuotas dev: 60 r/s con burst 120. PR2 leerá los límites desde Vault
//! `secret/consumers/<id>` para diferenciar consumidores ofensivos / pipeline CI.

use std::num::NonZeroU32;
use std::sync::Arc;

use dashmap::DashMap;
use governor::{
    clock::DefaultClock,
    state::{InMemoryState, NotKeyed},
    Quota, RateLimiter as Governor,
};

type ConsumerLimiter = Governor<NotKeyed, InMemoryState, DefaultClock>;

#[derive(Clone)]
pub struct RateLimiter {
    limiters: Arc<DashMap<String, Arc<ConsumerLimiter>>>,
    default_quota: Quota,
}

impl RateLimiter {
    pub fn new(per_second: u32, burst: u32) -> Self {
        let per = NonZeroU32::new(per_second.max(1)).expect("per_second > 0");
        let burst = NonZeroU32::new(burst.max(per.get())).expect("burst >= per_second");
        let quota = Quota::per_second(per).allow_burst(burst);
        Self {
            limiters: Arc::new(DashMap::new()),
            default_quota: quota,
        }
    }

    /// Devuelve `true` si la request cabe en la cuota, `false` si está bloqueada.
    /// El middleware traduce `false` a `Error::RateLimited`.
    pub fn check(&self, consumer_id: &str) -> bool {
        let limiter = self
            .limiters
            .entry(consumer_id.to_string())
            .or_insert_with(|| Arc::new(Governor::direct(self.default_quota)))
            .clone();
        limiter.check().is_ok()
    }
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new(60, 120)
    }
}
