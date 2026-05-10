# Wire API · TrenchPass

> Versión PR1 (placeholder routes). El contrato MCP nativo se activa en PR2
> sobre `transport-streamable-http-server` de `rmcp`.

## Convenciones

- Base URL: `https://trenchpass.<env>.alexendros.me`.
- `Content-Type: application/json` salvo healthchecks.
- Todas las respuestas de error tienen el shape:
  ```json
  { "error": "<código>", "message": "<descripción humana>" }
  ```

## Headers obligatorios para `/tool/*`

| Header | Formato | Origen |
|--------|---------|--------|
| `Authorization: Bearer <token>` | UUID v4 emitido por Vault | `secret/consumers/<id>` |
| `X-Forwarded-Tls-Client-Cert` | URI-encoded PEM cliente | inyectado por Traefik tras validar mTLS |
| `X-TrenchPass-Nonce` | UUID v4 único por request | generado por consumidor |
| `X-TrenchPass-Timestamp` | unix epoch (s) | generado por consumidor, ±5 min skew |

## Códigos de error estables

| HTTP | `error` | Significado |
|:----:|---------|-------------|
| 401 | `unauthorized` | Bearer ausente o inválido |
| 401 | `missing_client_cert` | mTLS requerido y header de cert ausente |
| 401 | `cert_revoked` | cert presente pero revocado en Vault PKI |
| 403 | `cn_mismatch` | CN del cert ≠ consumer del bearer |
| 403 | `scope_violation` | tool fuera del scope concedido |
| 404 | `not_found` | tool no registrada |
| 409 | `replay_detected` | nonce ya visto |
| 429 | `rate_limited` | cuota agotada |
| 502 | `upstream_error` | proveedor externo respondió error |
| 500 | `internal_error` | bug del gateway |

## Endpoints actuales (PR1)

### `GET /healthz`

```bash
curl https://trenchpass.dev.alexendros.me/healthz
# 200 OK · "ok"
```

### `GET /readyz`

```bash
curl https://trenchpass.dev.alexendros.me/readyz
# 200 OK · {"status":"ready","version":"0.1.0"}
```

### `GET /tools`

```bash
curl --cert ci.crt --key ci.key \
  -H "Authorization: Bearer $CTL_MCP_TOKEN" \
  https://trenchpass.dev.alexendros.me/tools
# 200 OK
# {
#   "namespaces": ["notion","stripe","github", …],
#   "tools": []   // PR1 stubs · PR2 puebla
# }
```

### `POST /tool/:name` (PR2 activa)

```bash
curl --cert ci.crt --key ci.key \
  -H "Authorization: Bearer $CTL_MCP_TOKEN" \
  -H "X-TrenchPass-Nonce: $(uuidgen)" \
  -H "X-TrenchPass-Timestamp: $(date +%s)" \
  -X POST \
  https://trenchpass.alexendros.me/tool/notion.search_pages \
  -d '{"query":"agentes","filter":{"value":"page","property":"object"}}'
# 200 OK
# {"results":[{ … }], "next_cursor":null}
```

## Scopes JSON

Un scope es un string `<namespace>:<verb>_<resource>` o `<namespace>:*` o `*`.

Ejemplo de fila Vault `secret/consumers/ci-staging`:

```json
{
  "scopes": ["notion:*", "stripe:list_*", "github:list_prs"],
  "ttl": "168h"
}
```

Reglas:

- `*` cubre cualquier tool.
- `<namespace>:*` cubre cualquier tool del namespace.
- exact match `<namespace>:<tool>` autoriza una sola.
- (PR3) prefijos parciales `<namespace>:read_*` (cuando se necesite).

## OpenAPI (PR2)

`packages/shared-types/openapi.yaml` se genera con:

```bash
cargo run --release -- --dump-schema > openapi.yaml
```

El schema incluye:

- los 70 tools con sus `params` y `result`.
- los 4 errores comunes (`unauthorized`, `scope_violation`, `rate_limited`,
  `replay_detected`).
- contracts para `/healthz`, `/readyz`, `/tools`, `/tool/{name}`, `/metrics`.

## Cliente TypeScript de referencia

Ver `packages/mcp-client-ts/src/index.ts` en el monorepo Controlink (PR6).
