# Runbook · Rotar el token de un proveedor

> Cuándo: rotación rutinaria (cada 90 d), sospecha de fuga, o tras incidente.

## Procedimiento

```bash
# 1. Generar el nuevo token en el proveedor (UI Stripe / Notion / GitHub).
#    Anota el TTL de validación si el proveedor lo permite.

# 2. Escribir en Vault.
vault kv put secret/providers/<provider> \
  token="<NUEVO_TOKEN>" \
  rotated_at="$(date -Iseconds)" \
  rotated_by="alexendros"

# 3. (Opcional) Forzar invalidación inmediata. Sin esto, llega al cache en ≤60 s.
curl -fsSL --cert admin.crt --key admin.key \
  -H "Authorization: Bearer $CTL_MCP_TOKEN_ADMIN" \
  -X POST https://trenchpass.alexendros.me/admin/invalidate \
  -d '{"path":"providers/<provider>"}'

# 4. Verificar smoke.
curl -fsSL --cert ci.crt --key ci.key \
  -H "Authorization: Bearer $CTL_MCP_TOKEN_CI" \
  -H "X-TrenchPass-Nonce: $(uuidgen)" -H "X-TrenchPass-Timestamp: $(date +%s)" \
  -X POST https://trenchpass.alexendros.me/tool/<provider>.list_<resource> \
  -d '{}'

# 5. Revocar el token antiguo en la UI del proveedor.

# 6. Esperar 5-10 min y revisar dashboard SigNoz "TrenchPass · Errors":
#    no debe haber spike de upstream_error con código 401 del proveedor.
```

## Verificación post-rotación

| Check | Cómo |
|-------|------|
| Cache del gateway al día | `cargo run -- diagnose vault-cache` (PR3) o `docker logs trenchpass` muestra el `cache miss` esperado. |
| Proton Pass espejo actualizado | Abrir bóveda `controlink-secrets` y verificar timestamp de la entrada. |
| Drift cero | dashboard `TrenchPass · Drift` queda en 0 tras la próxima ejecución horaria. |
| Audit registra la rotación | Query: `SELECT * FROM audit_events WHERE action='rotate_secret' ORDER BY ts DESC LIMIT 1;` |

## Rollback

Si el nuevo token falla:

```bash
# Reescribir con la versión anterior.
vault kv rollback -version=<N-1> secret/providers/<provider>
# El cache lo recogerá en ≤60 s o forzar invalidación.
```

## Bitácora

```
[YYYY-MM-DD HH:MM] alexendros: rotated providers/<provider> from kv-v2 vN-1 → vN. Smoke passed. SigNoz dashboard sano.
```
