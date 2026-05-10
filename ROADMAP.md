# Roadmap

> Snapshot vivo. La verdad operacional está en
> `cuadernos/agente__SuperKrabo_Controlink-MCP-arquitecto-gateway-vault-mtls/`
> (lado Alexendros). Este archivo es la cara pública del plan.

## Estado actual (snapshot 2026-05-10)

| PR | Hito | Estado |
|----|------|--------|
| **PR1** | Scaffold (Cargo workspace, axum router vacío, otel boot, vault client cache) | ✅ merged |
| **PR2** | Tools `notion` + `stripe` + `github`, audit Postgres en caliente | ✅ merged (PR2a #13 · PR2b #14 · PR2c #16) |
| **PR3** | mTLS rustls 0.23 (aws-lc-rs) + Vault PKI con TTL ≤ 7 d + custom Accept peer-cert wiring | ✅ merged (#17 base · #18 hardening · #19 peer-cert · #20 OTel 0.31 migration) |
| **PR4** | Proton Pass sync worker + drift detection (manifest YAML) | ✅ merged (#21) |
| **PR5** | 10 namespaces restantes (forgejo, dokploy, hostinger, vercel, n8n, glitchtip, docuseal, proton, gocardless_dd, gocardless_psd2) | ✅ merged (#22 base · #23 hardening · #24 test fidelity) |
| PR6 | `packages/mcp-client-ts` + migración 16 fetchers + 10 scripts del monorepo Controlink | 🟡 pendiente · cutover ~4 h |
| PR7 | Vía-fax (IMAP Proton + verificación PGP + handlers) | 🟡 pendiente |
| PR8 | ManitasFritas (heartbeat 30 d → alerta 60 d → disparo 90 d) + Shamir 3-de-5 | 🟡 pendiente |
| PR9 | Cleanup: `/source` AGPL §13, retirar Sentry/Glitchtip si SigNoz cubre, retirar env vars proveedor de Controlink, renombrar propuestas | 🟡 pendiente |

### Sub-PRs completados (no en grilla principal)

| Sub-PR | Hito | Estado |
|--------|------|--------|
| PR3.1 (#18) | Hardening TLS · trust anchors siempre refrescados, retry backoff 30s tras Vault outage, otel::shutdown via RAII guard | ✅ |
| PR3.2 (#19) | `transport::peer_cert::PeerCertAcceptor` · custom Accept que extrae cert del handshake rustls e inyecta en `Request::extensions()` · `auth::mtls::extract_cn` dispatcher direct/Traefik · `percent_encoding` upgrade | ✅ |
| PR3.3 (#20) | OTel stack 0.26 → 0.31 (builder API · `SdkTracerProvider` · `Resource::builder`) · drop feature `rt-tokio` | ✅ |
| PR3.4 (#23) | PR5 hardening · empty-string guards (`gocardless_psd2`, `glitchtip`), `AuthScheme` enum refactor (B2 design), tests por namespace | ✅ |
| PR3.5 (#24) | Test fidelity · extracción de guards a fns compartidas handler↔test · `AuthScheme: PartialEq, Eq` · named span retro-compat en `bearer_get_json` wrapper | ✅ |

## Hitos por trimestre

### Q2-2026 (en curso)
- ✅ PR0 hook canon
- ✅ PR1 scaffold
- ✅ PR2 (a/b/c) tools notion + stripe + github + audit Postgres
- ✅ PR3 (+3.1/3.2/3.3) mTLS + peer-cert wiring + OTel 0.31
- ✅ PR4 sync worker drift detection
- ✅ PR5 (+3.4/3.5) 10 namespaces restantes + hardening
- 🟡 Targeted milestone: **gateway en producción con 13 namespaces vivos** antes del 2026-06-15 (sólo falta PR6 cutover Controlink).

### Q3-2026
- 🟡 PR6 cutover Controlink (4 h downtime planificado en sábado).
- Auditoría externa de auth/mTLS (terceros · paga Alexendros).

### Q4-2026
- PR7 vía-fax estabilizado tras drill controlado.
- PR8 ManitasFritas con drill semestral con custodios reales.
- PR9 cleanup + endpoint `/source` AGPL.
- Considerar **`trenchpass-cli`** (binario auxiliar para operadores: rotación, status, smoke).

### 2027+
- Multi-tenant: cada cliente con su sub-mount Vault, su sub-CA y su namespace en SigNoz.
- Generación automática de `mcp-client-py` (Python) y `mcp-client-go`.
- Federation entre instancias (gateway-of-gateways) si la flota crece a >5 productos.

## Desiderata (sin compromiso de fecha)

- WASM plugin host para tools de cliente.
- Soporte de OAuth dynamic client registration (rmcp ya tiene base).
- Dashboard web embebido (HTMX + tower-livereload) que sustituya el `/tools` JSON.
- Hardware token de unseal Vault (YubiKey en lugar de shard papel).
