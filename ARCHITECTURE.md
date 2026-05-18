# Arquitectura · TrenchPass

> Documento vivo. Cualquier cambio sustancial requiere un ADR en `docs/adr/`.

## 1. Visión

TrenchPass es **un único punto de custodia** para los ~20 secretos de proveedor
externo que hoy están dispersos en 16 fetchers TypeScript del monorepo
[Controlink](https://github.com/Alexendros/Controlink). En vez de inyectar
`STRIPE_API_KEY`, `NOTION_TOKEN`, `GITHUB_TOKEN` … en cada app, las apps llaman
**una sola API MCP**, y TrenchPass traduce esas llamadas a peticiones reales al
proveedor con el secreto vivo que recupera de Vault.

```
┌──────────────┐   mTLS + Bearer    ┌───────────────────────┐
│  Consumidor  │ ─────────────────► │       TrenchPass        │
│  (Next.js,   │                    │  axum + rmcp + tools  │
│   CI, n8n…)  │ ◄───────────────── │                       │
└──────────────┘   JSON / SSE       └────────┬──────────────┘
                                             │
              ┌──────────────────────────────┼──────────────────────────────┐
              │                              │                              │
              ▼                              ▼                              ▼
     ┌─────────────────┐          ┌────────────────────┐         ┌──────────────────┐
     │   Vault OSS     │          │ otel-collector →   │         │  Postgres audit  │
     │ KV v2 + PKI     │          │ SigNoz             │         │  append-only     │
     └────────┬────────┘          └────────────────────┘         └──────────────────┘
              │
              ▼
     ┌─────────────────┐
     │  Proton Pass    │  ← espejo offline + recovery
     │  (Swiss vault)  │
     └─────────────────┘
```

## 2. Garantías de diseño

| Garantía | Mecanismo |
|----------|-----------|
| Custodia única | Vault OSS hot path, Proton Pass espejo. Cero secretos en `.env` de consumidores. |
| Rotación sin redeploy | `vault kv put secret/providers/<x> token=NEW` → cache 60 s lo recoge. |
| Doble factor | Bearer token con scopes JSON **+** cert mTLS (CN = consumer ID). |
| Audit immutable | Postgres con rol `audit_writer` que solo tiene `INSERT`. |
| Observabilidad de origen | `tracing-opentelemetry` → otel-collector → SigNoz (traces, metrics, logs). |
| Recovery k-de-n | Shamir 3-de-5 distribuidos: Proton Pass · YubiKey · papel · custodio externo · R2 cifrado. |
| Comando out-of-band | «Vía-fax»: mail PGP firmado al buzón Proton del gateway. |
| Dead-hand switch | «ManitasFritas»: heartbeat 30 d → alerta 60 d → disparo 90 d. |
| Network-service AGPL | Endpoint `/source` (PR9) con tarball del HEAD. |

## 3. Capas

### 3.1 Transport
- HTTPS terminado en Traefik (TLS 1.3) con `infra-mtls` activo.
- El cert del cliente se pasa al gateway en `X-Forwarded-Tls-Client-Cert` (URI-encoded).
- En PR3 ofrecemos también `transport::mtls` con `tokio-rustls` para escenarios sin Traefik.
- MCP framing: `transport-streamable-http-server` de `rmcp` 1.7.

### 3.2 Auth (`src/auth/`)
1. `bearer::extract` → `Authorization: Bearer <token>`.
2. `bearer::resolve` (PR2) → mira `secret/consumers/<id>` en Vault y obtiene scopes.
3. `mtls::extract_cn` + `mtls::assert_match` → CN del cert == consumer del bearer.
4. `scope::check` → autoriza la tool concreta.

### 3.3 Security middleware (`src/security/`)
| Capa | Propósito | Crate |
|------|-----------|-------|
| RateLimiter | Token bucket por consumidor | `governor` |
| ReplayCache | Nonce + timestamp ±5 min | `dashmap` |
| auth_middleware | Compone bearer → mTLS → rate → replay | `axum::middleware::from_fn_with_state` |

### 3.4 Vault client (`src/vault/`)
- `vaultrs::kv2` para KV v2 (mount=`secret`).
- Cache `DashMap<String, Secret>` con TTL configurable (`VAULT_CACHE_TTL_SECS`, default 60 s).
- `invalidate(path)` y `invalidate_all()` para forzar refresh tras rotación.

### 3.5 Audit (`src/audit/`)
- Schema en `sql/init_audit.sql` (co-vendorizado con Controlink).
- Append-only por privilegios Postgres: el rol `audit_writer` solo tiene `INSERT`.
- Sin `RETURNING` (requeriría `SELECT`); el dashboard Controlink lee con `audit_reader`.

### 3.6 Tools (`src/tools/`)
- 13 namespaces: notion, stripe, github, forgejo, dokploy, hostinger, vercel, n8n,
  glitchtip, docuseal, proton, gocardless_dd, gocardless_psd2.
- Convención: `<namespace>.<verb>_<resource>` (ej. `notion.search_pages`).
- Cada namespace devuelve `Vec<ToolDef>` desde `pub fn tools()`.
- PR2 implementa los tres primeros; PR5 el resto.

### 3.7 Observabilidad (`src/otel/`)
- Pipeline OTLP gRPC → otel-collector → SigNoz (ClickHouse).
- Recursos: `service.name=trenchpass`, `service.namespace=alexendros`,
  `deployment.environment=<env>`.
- Sampling: `parentbased_traceidratio` (PR2 — 1.0 dev, 0.1 prod).

### 3.8 Recovery (PR4 + PR8)
- **Proton Pass sync worker**: cada `kv put` en Vault dispara escritura espejada en Proton Pass `controlink-secrets`. Lag <30 s.
- **Drift detection**: cron horario compara hash de la bóveda Proton con snapshot Vault. Discrepancia → alerta SigNoz.
- **Shamir 3-de-5**: la unseal key se parte en 5 shards distribuidos.

## 4. Flujo de una llamada (`stripe.list_subscriptions`)

```
Consumidor                Traefik           TrenchPass              Vault            Stripe
    │                        │                 │                    │                 │
    │ POST /tool/stripe.…    │                 │                    │                 │
    │ + cert + Bearer        │                 │                    │                 │
    ├───────────────────────►│ valida cliente  │                    │                 │
    │                        ├────────────────►│ middleware:        │                 │
    │                        │                 │  bearer→mtls→rate  │                 │
    │                        │                 │  →replay           │                 │
    │                        │                 │ tools::dispatch    │                 │
    │                        │                 ├───────────────────►│ kv2 read        │
    │                        │                 │                    │ secret/         │
    │                        │                 │◄───────────────────┤ providers/      │
    │                        │                 │   {api_key: …}     │ stripe          │
    │                        │                 │                    │                 │
    │                        │                 ├──────────────────────────────────────►│
    │                        │                 │ GET /v1/subscriptions  + Bearer       │
    │                        │                 │◄──────────────────────────────────────┤
    │                        │                 │                                       │
    │                        │                 ├ audit::record(Ok, latency_ms)         │
    │                        │                 ├ trace span exportado                  │
    │                        │◄────────────────┤  200 OK + JSON                        │
    │◄───────────────────────┤                 │                                       │
```

## 5. Decisiones clave (resumen ADR)

| ADR | Decisión |
|-----|----------|
| 0001 | Lenguaje gateway = Rust (footprint, type safety, ecosistema rmcp). |
| 0002 | Repo satélite (Cargo workspace) en lugar de monorepo Turborepo (Cargo no encaja en Turbo). |
| 0003 | Vault OSS hot path + Proton Pass espejo (papel autocopiante). |
| 0004 | mTLS terminado en Traefik (un único punto TLS) + acceptor rustls como fallback. |
| 0005 | Audit en Postgres dedicado (`postgres-audit`), no SQLite ni el Postgres de Controlink. |
| 0006 | Observabilidad SigNoz desde día uno, Sentry/Glitchtip se retiran en PR9 si SigNoz cubre. |
| 0007 | Vía-fax como canal redundante PGP-firmado para comandos críticos. |
| 0008 | ManitasFritas Shamir 3-de-5 (continuidad de servicio sin SPoF humano). |

ADRs completos en `docs/adr/`.

## 6. Footprint

| Recurso | Estimado |
|---------|----------|
| Imagen Docker (distroless/cc) | ~12 MB |
| RSS en idle | <30 MB |
| RSS bajo carga (200 r/s) | <120 MB |
| Latencia p50 (cache hit) | <2 ms gateway, <15 ms total con Stripe |
| Latencia p99 (cache miss) | <50 ms gateway |

## 7. No-objetivos

- **No** es un proxy genérico HTTP — solo enruta MCP tools declaradas.
- **No** custodia secretos *internos* de Controlink (`AUTH_SECRET`,
  webhooks signing, password hashes). Esos siguen en Vault de Controlink directamente.
- **No** sustituye a Vault — lo complementa con cache + ergonomía MCP.
- **No** corre como sidecar de los consumidores; es un servicio independiente
  detrás de Traefik.
