//! Capas defensivas previas al tool router: rate limit, replay protection,
//! middleware de auth + audit.

pub mod middleware;
pub mod ratelimit;
pub mod replay;

pub use middleware::auth_middleware;
pub use ratelimit::RateLimiter;
pub use replay::ReplayCache;
