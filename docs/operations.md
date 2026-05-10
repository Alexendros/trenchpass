# Operaciones · TrenchPass

> Manual del operador. Asume que TrenchPass está desplegado en Dokploy detrás
> de Traefik con Vault y Postgres-audit como sidecars en el mismo VPS.

## Topología actual (2026-05)

```
                        ┌────────────┐
   tráfico público ────►│ Cloudflare │──┐
                        └────────────┘  │
                                        ▼
                                  ┌─────────┐
                                  │ Traefik │  TLS 1.3 + mTLS terminate
                                  └────┬────┘
                                       │
                ┌──────────────────────┼──────────────────────┐
                ▼                      ▼                      ▼
        ┌──────────────┐       ┌──────────────┐      ┌──────────────┐
        │  controlink  │       │  trenchpass    │      │ otros consum │
        │  (Next.js)   │       │  (gateway)   │      │              │
        └──────┬───────┘       └──────┬───────┘      └──────────────┘
               │                      │
               │   HTTPS+SSE          │
               └─────────────────────►│
                                      │
              ┌───────────────────────┼─────────────────────┐
              ▼                       ▼                     ▼
       ┌─────────────┐         ┌─────────────┐      ┌──────────────┐
       │  Vault OSS  │         │ Postgres    │      │ otel-collector│
       │  KV + PKI   │         │  audit      │      │   → SigNoz    │
       └─────────────┘         └─────────────┘      └──────────────┘
```

## Endpoints

| Endpoint | Auth | Propósito |
|----------|------|-----------|
| `GET /healthz` | none | liveness |
| `GET /readyz` | none | readiness (Vault + Postgres reachable) |
| `GET /tools` | bearer + cert | listado de tools disponibles |
| `POST /tool/:name` | bearer + cert | invocación de tool |
| `GET /metrics` (PR2) | none, restricción red | Prometheus / OTel mirror |
| `GET /source` (PR9) | none | tarball del HEAD (cumplimiento AGPL §13) |

## Variables de entorno principales

Ver `.env.example`. Las críticas:

| Variable | Default | Notas |
|----------|---------|-------|
| `TRENCHPASS_BIND` | `0.0.0.0:8300` | el binario solo escucha en red interna del compose |
| `TRENCHPASS_ENV` | `development` | `production` activa validaciones extra |
| `TRENCHPASS_MTLS_REQUIRED` | `false` (PR1) → `true` (PR3) | en prod siempre `true` |
| `VAULT_TOKEN` | obligatorio | AppRole token con policy mínima |
| `VAULT_CACHE_TTL_SECS` | `60` | propagación máxima post-rotación |
| `DATABASE_URL` | obligatorio | rol `audit_writer` |
| `OTEL_EXPORTER_OTLP_ENDPOINT` | `http://otel-collector:4317` | red interna |

## Procedimientos comunes

### Rotar el token de un proveedor

```bash
# 1. Obtener nuevo token del proveedor (UI Stripe / Notion / …)
# 2. Escribir en Vault
vault kv put secret/providers/notion token=ntn_NEW

# 3. (opcional) forzar invalidación del cache
curl -fsSL --cert client.crt --key client.key \
  -H "Authorization: Bearer $CTL_MCP_TOKEN_ADMIN" \
  -X POST https://trenchpass.alexendros.me/admin/invalidate \
  -d '{"path":"providers/notion"}'

# 4. Verificar
curl -fsSL --cert client.crt --key client.key \
  -H "Authorization: Bearer $CTL_MCP_TOKEN_CI" \
  -X POST https://trenchpass.alexendros.me/tool/notion.search_pages \
  -d '{"query":"smoke"}'
```

### Emitir cert para un nuevo consumidor

```bash
vault write -format=json pki/issue/trenchpass-consumers \
  common_name="ci-staging" \
  ttl="168h" \
  > certs/ci-staging.json

jq -r .data.certificate    certs/ci-staging.json > certs/ci-staging.crt
jq -r .data.private_key    certs/ci-staging.json > certs/ci-staging.key
jq -r .data.issuing_ca     certs/ci-staging.json > certs/vault-ca.crt
```

### Revocar un consumer

```bash
# Revocar cert vía vía-fax (PGP-firmado), o directamente:
vault write pki/revoke serial_number=<serial>

# Tarda <60 s en propagarse a la CRL que Traefik consume.
# Auditoría: SigNoz dashboard "TrenchPass · Revocations".
```

### Backup audit log

```bash
# Cron horario en host:
pg_dump -h postgres-audit -U audit_admin audit \
  | gpg --encrypt --recipient operator@proton.me \
  | rclone rcat r2:trenchpass-backups/audit/$(date -Iseconds).sql.gpg
```

## Alertas SigNoz (configurar tras PR2)

| Alerta | Severidad | Trigger |
|--------|-----------|---------|
| `vault_unreachable` | P1 | 5 min sin `vault.read` exitoso |
| `audit_write_failures` | P0 | >0 INSERT failed en 1 min |
| `scope_violations` | P2 | >5 en 5 min para mismo `consumer_id` |
| `replay_detected` | P1 | >3 en 1 min |
| `proton_pass_drift` | P2 | drift hash detectado en 2 cron consecutivos |
| `manitasfritas_alert_window` | P1 | >60 d sin heartbeat |

## Dashboards SigNoz

- `TrenchPass · Overview` — RPS, p50/p99, error rate, by namespace.
- `TrenchPass · Vault` — cache hit rate, TTL distribution, sealed?.
- `TrenchPass · Audit` — eventos por consumer, outcomes, anomalías.
- `TrenchPass · ManitasFritas` — heartbeat counter, ventanas alert/disparo.

Spec exportado en `infra/signoz/dashboards/` (PR2).

## Disaster recovery

Lee `docs/runbooks/recovery-drill.md` y `docs/runbooks/manitasfritas/operator.md`
(PR8). Resumen:

1. Vault corrupto → restore desde Proton Pass o desde shards Shamir.
2. Postgres-audit perdido → restore desde dump R2 cifrado.
3. Container gateway down → Dokploy redeploy automático <60 s.
4. Operador caído permanentemente → ManitasFritas activa el plan a 90 d.
