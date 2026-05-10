# TrenchPass

**MCP gateway · custodio único de credenciales** para el ecosistema Alexendros (Controlink y satélites).
Centraliza ~20 secretos de proveedor externo (Notion, Stripe, GitHub, Forgejo, Dokploy, Hostinger, Vercel, n8n, GlitchTip, DocuSeal, Proton, GoCardless DD/PSD2) detrás de un único endpoint MCP con doble factor (Bearer + mTLS), audit append-only y observabilidad SigNoz.

> **Estado**: PR1 · scaffold. Compila pero no enruta tools (PR2 — Notion + Stripe + GitHub).

---

## Arquitectura (resumen)

```
Consumidores (cert mTLS + Bearer + scope)
   ↓ HTTPS+SSE (Traefik termina TLS, propaga cert)
TrenchPass (Rust · axum + rmcp)
   ├── Vault OSS  ← runtime hot path (cache 60 s)
   ├── Proton Pass ← espejo + recovery (PR4)
   ├── Postgres   ← audit_events append-only
   └── otel-collector → SigNoz
```

Diseño completo: `propuesta_#95_controlink_mcp-gateway-rust-vault-protonpass.md` y
`propuesta_#96_trenchpass_mcp-gateway-rust-vault-standalone.md`.

## Estructura

```
src/
├── main.rs            boot: config → otel → AppState → axum serve
├── lib.rs             AppState compartido (config, vault, audit, tools, rate, replay)
├── config.rs          carga `.env` / entorno
├── error.rs           Error enum + IntoResponse
├── otel/setup.rs      OTLP gRPC → SigNoz
├── vault/client.rs    `vaultrs` + cache `DashMap` 60 s
├── audit/store.rs     sqlx INSERT append-only en `audit_events`
├── auth/{bearer,mtls,scope}.rs   doble factor + scopes JSON
├── security/{ratelimit,replay,middleware}.rs   token bucket + nonce + auth layer
├── transport/{sse,mtls}.rs       router HTTP (mTLS rustls llega en PR3)
└── tools/             13 namespaces · stubs PR1
```

## Build

```bash
# toolbox / distrobox con Rust 1.82+
toolbox enter rust    # o cualquier contenedor con rustup
cargo build --release
```

Imagen Docker:

```bash
docker build -t ghcr.io/alexendros/trenchpass:dev .
```

## Run (desarrollo local)

```bash
cp .env.example .env
# Rellena VAULT_TOKEN, DATABASE_URL, TRENCHPASS_DEV_BEARER
cargo run
curl http://localhost:8300/healthz
curl -H "Authorization: Bearer $TRENCHPASS_DEV_BEARER" http://localhost:8300/tools
```

## Roadmap PRs

| PR | Contenido | Estado |
|----|-----------|--------|
| **PR1** | Scaffold (este commit) | en curso |
| PR2 | Tools `notion` + `stripe` + `github` + audit Postgres | pendiente |
| PR3 | mTLS rustls + Vault PKI | pendiente |
| PR4 | Proton Pass sync worker + drift detection | pendiente |
| PR5 | 10 namespaces restantes | pendiente |
| PR6 | `packages/mcp-client-ts` + migración fetchers Controlink | pendiente |
| PR7 | Vía-fax (IMAP + PGP + handlers) | pendiente |
| PR8 | ManitasFritas (heartbeat + disparo + Shamir) | pendiente |
| PR9 | Cleanup retirar Sentry, retirar env vars de Controlink | pendiente |

## Notas técnicas

- **`rmcp` 1.6** (latest crates.io). Habilitamos `transport-streamable-http-server` (sustituye
  al SSE clásico conforme a la revisión MCP 2025-03+); el plan `propuesta_#96` referenciaba
  `rmcp = "0.x"` antes de la 1.0 — adaptamos a la API actual.
- **Workspace single-crate**. Si en el futuro extraemos un `crates/sdk` o
  `crates/protocol`, el `[workspace]` ya está preparado.
- **`sqlx::query_scalar` runtime-checked**. Evitamos `cargo sqlx prepare` en CI offline;
  cuando el job de tests tenga DB lista, podemos migrar a `query!` con compile-time check.
- **mTLS terminado en Traefik**. El binario lee `X-Forwarded-Tls-Client-Cert` (PR3).
  Para escenarios sin Traefik, `transport::mtls::build` proporciona acceptor rustls.

## Documentación

### Raíz (canon)

| Archivo | Contenido |
|---------|-----------|
| [`ARCHITECTURE.md`](ARCHITECTURE.md) | Visión, capas, flujo de una llamada, footprint, no-objetivos. |
| [`COMPARISON.md`](COMPARISON.md) | Comparativa con Vault Agent, OpenBao, Infisical, Doppler, mcp-proxy, Smithery, Pomerium… |
| [`ROADMAP.md`](ROADMAP.md) | PRs y hitos por trimestre. |
| [`RELEASE.md`](RELEASE.md) | SemVer aplicado, cosign, hotfix, calendario. |
| [`SECURITY.md`](SECURITY.md) | Reportes, modelo de amenazas resumen, hardening checklist. |
| [`CONTRIBUTING.md`](CONTRIBUTING.md) | Setup, flujo, commits, estilo, testing, revisión. |
| [`STYLEGUIDE.md`](STYLEGUIDE.md) | Convenciones Rust idioma, naming, errores, async, tracing. |
| [`SUPPORT.md`](SUPPORT.md) | Canales, expectativas, ausencia de SLA. |
| [`MAINTAINERS.md`](MAINTAINERS.md) | Roles, sucesión, conflicto de interés. |
| [`AUTHORS.md`](AUTHORS.md) | Lead + contribuyentes + reconocimientos. |
| [`COPYRIGHT.md`](COPYRIGHT.md) | AGPL §13, third-party notices, trademarks. |
| [`CHANGELOG.md`](CHANGELOG.md) | Keep a Changelog · SemVer. |
| [`CODE_OF_CONDUCT.md`](CODE_OF_CONDUCT.md) | Contributor Covenant 2.1. |
| [`CITATION.cff`](CITATION.cff) | Metadatos para citar el software. |

### `docs/`

| Archivo | Contenido |
|---------|-----------|
| [`docs/operations.md`](docs/operations.md) | Manual del operador, alertas, procedimientos. |
| [`docs/api.md`](docs/api.md) | Wire API: headers, scopes, códigos de error, ejemplos. |
| [`docs/threat-model.md`](docs/threat-model.md) | STRIDE + LINDDUN extendido. |
| [`docs/glossary.md`](docs/glossary.md) | Vocabulario controlado. |
| [`docs/adr/`](docs/adr/) | 8 ADRs aceptados (lenguaje, repo, vault híbrido, mTLS Traefik…). |
| [`docs/runbooks/`](docs/runbooks/) | vault-unseal, rotate-provider-token, revoke-consumer, recovery-drill, manitasfritas, incident-response, postmortem-template. |

## Licencia

AGPL-3.0-or-later · © Alexendros 2026. Texto íntegro en [`LICENSE`](LICENSE).
