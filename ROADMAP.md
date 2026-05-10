# Roadmap

> Snapshot vivo. La verdad operacional está en
> `cuadernos/agente__SuperKrabo_Controlink-MCP-arquitecto-gateway-vault-mtls/`
> (lado Alexendros). Este archivo es la cara pública del plan.

## Estado actual

| PR | Hito | Estado |
|----|------|--------|
| **PR1** | Scaffold (Cargo workspace, axum router vacío, otel boot, vault client cache) | ✅ en revisión |
| PR2 | Tools `notion` + `stripe` + `github`, audit Postgres en caliente | 🟡 en curso (PR2a ✅ audit · PR2b 🟡 tools notion · PR2c 🟡 stripe+github) |
| PR3 | mTLS rustls + Vault PKI con TTL ≤ 7 d | 🟡 pendiente |
| PR4 | Proton Pass sync worker + drift detection | 🟡 pendiente |
| PR5 | 10 namespaces restantes (forgejo, dokploy, hostinger, vercel, n8n, glitchtip, docuseal, proton, gocardless_dd, gocardless_psd2) | 🟡 pendiente |
| PR6 | `packages/mcp-client-ts` + migración 16 fetchers + 10 scripts del monorepo Controlink | 🟡 pendiente · cutover ~4 h |
| PR7 | Vía-fax (IMAP Proton + verificación PGP + handlers) | 🟡 pendiente |
| PR8 | ManitasFritas (heartbeat 30 d → alerta 60 d → disparo 90 d) + Shamir 3-de-5 | 🟡 pendiente |
| PR9 | Cleanup: `/source` AGPL §13, retirar Sentry/Glitchtip si SigNoz cubre, retirar env vars proveedor de Controlink, renombrar propuestas | 🟡 pendiente |

## Hitos por trimestre

### Q2-2026 (en curso)
- ✅ PR0 hook canon
- 🟡 PR1 scaffold (este sprint)
- 🟡 PR2 + PR3 (sprint siguiente)
- Targeted milestone: **gateway en producción con 3 namespaces vivos** antes del 2026-06-15.

### Q3-2026
- PR4 sync worker (estabiliza recovery).
- PR5 namespaces restantes.
- PR6 cutover Controlink (4 h downtime planificado en sábado).
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
