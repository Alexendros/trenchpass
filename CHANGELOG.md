# Changelog

Sigue [Keep a Changelog](https://keepachangelog.com/es/1.1.0/) y
[SemVer](https://semver.org/lang/es/).

## [Unreleased]

### Added · PR7 vía-fax (canal PGP out-of-band)

- `src/fax/{mod,pgp,commands,dispatch,imap}.rs` · worker IMAP que polea
  Proton (`imap.protonmail.ch:993` por defecto), verifica firma PGP del
  operador con `sequoia-openpgp` y despacha comandos atómicos.
- Verbos soportados:
  - `invalidate-all` ✅ limpia el cache local de `VaultClient`.
  - `invalidate` ✅ limpia una entrada KV concreta.
  - `revoke` / `seal-vault` ⏳ stubs con `FaxError::Dispatch` · cableado a
    `vaultrs::pki::cert::revoke` y `vaultrs::sys::seal` queda para PR7.1.
- Anti-replay reusa `security::ReplayCache` (timestamp ±5 min, nonce único).
- Audit log en `audit_events` con `consumer_id='via-fax'`, `action='fax.<verb>'`
  y `detail.signature_sha256` para correlación forense.
- `AppState::fax_operator_cert` cargado de `FAX_PGP_OPERATOR_CERT_PATH`
  (export armored OpenPGP, no PEM x509).
- Worker arrancado en `main.rs` sólo si la config está completa; ausencia
  de config no es error.
- Deps: `sequoia-openpgp 1` (feature `crypto-openssl`), `async-imap 0.10`,
  `mailparse 0.15`.
- 17 tests inline · `cargo test --lib fax` verde · 87/87 totales.
- Runbook: `docs/runbooks/via-fax.md` (gpg --sign --armor, troubleshooting).
- Example offline: `cargo run --example fax_smoke`.

### Changed

_Sin cambios registrados en esta línea._

### Deprecated

_Sin entradas obsoletas registradas._

### Removed

_Sin elementos eliminados registrados._

### Fixed

_Sin correcciones registradas (ver «Verified» de 0.1.0 para fixes del scaffold)._

### Security

_Sin avisos de seguridad pendientes; ver `SECURITY.md` para la política de divulgación._

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
