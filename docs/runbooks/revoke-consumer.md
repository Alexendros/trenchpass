# Runbook · Revocar un consumer (Bearer + cert)

> Cuándo: sospecha de Bearer filtrado, compromiso del host del consumer, o
> consumer ya no operativo.

## Acción inmediata (≤2 min)

```bash
# 1. Listar el cert vigente del consumer.
vault list -format=json pki/certs | jq '.[] | select(.cn=="<consumer-id>")'

# 2. Revocar por número de serie.
vault write pki/revoke serial_number=<serial>

# 3. Forzar al CRL distribuido a Traefik (si CRL automática no es <1 min):
curl -fsSL --cert admin.crt --key admin.key \
  -H "Authorization: Bearer $CTL_MCP_TOKEN_ADMIN" \
  -X POST https://trenchpass.alexendros.me/admin/refresh-crl

# 4. Eliminar el Bearer del Vault del consumer.
vault delete secret/consumers/<consumer-id>

# 5. Forzar invalidación de cache.
curl … /admin/invalidate -d '{"path":"consumers/<consumer-id>"}'

# 6. Verificar que el consumer recibe 401.
curl --cert <consumer-id>.crt --key <consumer-id>.key \
  -H "Authorization: Bearer $OLD_BEARER" \
  https://trenchpass.alexendros.me/tools
# 401 unauthorized
```

## Acción de seguimiento (≤24 h)

1. Audit timeline:
   ```sql
   SELECT * FROM audit_events
   WHERE consumer_id = '<consumer-id>'
   AND ts > NOW() - INTERVAL '7 days'
   ORDER BY ts DESC;
   ```
2. Buscar acciones sospechosas en SigNoz `TrenchPass · Forensics` por consumer.
3. Si se detecta abuso, reportar a `SECURITY.md` workflow.

## Re-emisión (si se trata de redeploy controlado)

```bash
# Nuevo cert con TTL corto inicialmente.
vault write -format=json pki/issue/trenchpass-consumers \
  common_name="<consumer-id>" \
  ttl="24h"

# Nuevo Bearer.
vault kv put secret/consumers/<consumer-id> \
  scopes='["notion:*"]' \
  ttl="168h"
```

## Bitácora

```
[YYYY-MM-DD HH:MM] alexendros: revoked consumer=<consumer-id> serial=<serial>. Reason: <fuga|deprecated|other>. New cert issued: <yes/no>.
```
