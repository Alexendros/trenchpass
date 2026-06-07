//! Transport layer: HTTPS+SSE primario (PR1), mTLS rustls (PR3), vía-fax (PR7).
//!
//! En PR1 montamos un `axum::Router` plano que expone:
//!   - `GET  /healthz`     → liveness
//!   - `GET  /readyz`      → readiness (Vault + Postgres reachable)
//!   - `POST /tool/:name`  → invocación de tool (route stub que devuelve 501)
//!
//! PR2 añade el handler MCP real sobre `transport-streamable-http-server`.

pub mod admin;
pub mod mtls;
pub mod peer_cert;
pub mod sse;

pub use peer_cert::{PeerCertAcceptor, PeerCertificate};
pub use sse::router;
