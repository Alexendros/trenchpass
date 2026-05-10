# Changelog

Sigue [Keep a Changelog](https://keepachangelog.com/es/1.1.0/) y
[SemVer](https://semver.org/lang/es/).

## [Unreleased]

## [0.1.0] · 2026-05-10 · PR1 scaffold

### Added
- Cargo workspace single-crate con dependencias pinneadas (rmcp 1.6, axum 0.7,
  tokio 1, vaultrs 0.7, sqlx 0.8, opentelemetry 0.26, governor 0.7).
- `src/main.rs` boot con shutdown signal handler (SIGINT + SIGTERM).
- `src/lib.rs` con `AppState` compartido por handlers axum.
- `src/config.rs` lee entorno (`.env` en dev) y bloquea `TRENCHPASS_DEV_BEARER`
  si `TRENCHPASS_ENV=production`.
- `src/error.rs` con `Error` + `IntoResponse` (mapeo a códigos HTTP estables).
- `src/otel/setup.rs` exporta traces vía OTLP gRPC al otel-collector.
- `src/vault/client.rs` con cache `DashMap` TTL 60 s (configurable).
- `src/audit/store.rs` con `AuditStore::record` append-only sin `RETURNING`
  (rol Postgres `audit_writer` no tiene `SELECT`).
- `src/auth/{bearer,mtls,scope}.rs` capa de autenticación doble factor + scopes
  con tests unitarios.
- `src/security/{ratelimit,replay,middleware}.rs` token bucket por consumidor +
  nonce/timestamp ±5 min + middleware que ata todo.
- `src/transport/{sse,mtls}.rs` router HTTP plano (`/healthz`, `/readyz`,
  `/tools`, `POST /tool/:name` stub PR2).
- `src/tools/` registry + 13 namespaces stub (notion, stripe, github, forgejo,
  dokploy, hostinger, vercel, n8n, glitchtip, docuseal, proton, gocardless_dd,
  gocardless_psd2).
- `Dockerfile` multi-stage `rust:1.82-bookworm` → `distroless/cc-debian12`,
  imagen final ~12 MB.
- GitHub Actions: `fmt`, `clippy`, `test` (con servicio Postgres + schema vendorizado),
  `docker` (push GHCR en main).
- `sql/init_audit.sql` vendorizado del schema Controlink (mismo `audit_events`).
- Documentación canon: `LICENSE` (AGPL-3.0 verbatim), `COPYRIGHT.md`,
  `CITATION.cff`, `ARCHITECTURE.md`, `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`,
  `SECURITY.md`, `ROADMAP.md`, `RELEASE.md`, `STYLEGUIDE.md`, `SUPPORT.md`,
  `MAINTAINERS.md`, `AUTHORS.md`, `COMPARISON.md`, `THREAT_MODEL.md` (PR1 stub),
  ADRs 0001–0008.

### Notes
- `rmcp` originalmente planteado en `0.x`; ajustado a la línea **1.6** estable.
- mTLS rustls acceptor en `transport::mtls` queda como stub que retorna error;
  PR3 lo activa cuando Traefik opcionalmente bypassee.
- `cargo check` requiere `protobuf-compiler` instalado en el entorno (Fedora:
  `dnf install protobuf-compiler`).

### Verified
- `cargo check` ✅ verde en `rustc 1.95.0` Fedora 43 (toolbx).
- `cargo check --locked` ✅ verde tras commit del `Cargo.lock` (440 packages
  resueltos en 5.6 s).
- Fixes aplicados sobre el scaffold inicial:
  - `src/otel/setup.rs`: `install_batch` en `opentelemetry-otlp` 0.26 devuelve
    `TracerProvider`, no `Tracer`. Ahora derivamos `Tracer` con `provider.tracer(name)`
    y registramos el provider globalmente.
  - `src/auth/mtls.rs`: el CN del cert se materializa como `String` propio
    antes de devolver para no propagar lifetimes prestados de `pem`/`cert`.

[Unreleased]: https://github.com/Alexendros/trenchpass/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/Alexendros/trenchpass/releases/tag/v0.1.0
