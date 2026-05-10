# Runbook · Vault unseal

> Cuándo: tras reinicio del VPS, redeploy del contenedor Vault, o evento de
> sealing manual (`vault operator seal`).

## Síntomas

- `trenchpass` devuelve `502 upstream_error` con `error_kind: vault_sealed`.
- `/readyz` del gateway: `503` con `{"vault":"sealed"}`.
- `vault status` muestra `Sealed: true`.

## Pre-requisitos

- Acceso SSH al VPS Hostinger.
- 3 de los 5 shards Shamir distribuidos.
- Token PGP del operador (para firmar la acción en bitácora).

## Procedimiento

```bash
# 1. SSH al VPS
ssh root@vps-controlink.alexendros.me

# 2. Entrar al contenedor Vault
docker exec -it vault sh
export VAULT_ADDR=http://localhost:8200

# 3. Verificar que está sellado
vault status
# Sealed             true
# Total Shares       5
# Threshold          3

# 4. Unseal con 3 shards (uno cada vez)
vault operator unseal <SHARD_1>
vault operator unseal <SHARD_2>
vault operator unseal <SHARD_3>
# Sealed             false

# 5. Verificar
vault status

# 6. Forzar al gateway a refrescar cache (opcional)
docker restart trenchpass

# 7. Smoke
curl -fsSL --cert ci.crt --key ci.key \
  -H "Authorization: Bearer $CTL_MCP_TOKEN_CI" \
  -H "X-TrenchPass-Nonce: $(uuidgen)" -H "X-TrenchPass-Timestamp: $(date +%s)" \
  -X POST https://trenchpass.alexendros.me/tool/notion.search_pages \
  -d '{"query":"smoke"}'
# 200 OK con resultados
```

## Post-acción

1. Registra en `cuadernos/agente__SuperKrabo_…/bitacora.md` con timestamp y
   firmado PGP.
2. Verifica dashboard SigNoz `TrenchPass · Vault` no muestre nuevas alertas.
3. Si el unseal fue inesperado, abre incidente P1.

## Troubleshooting

- **`vault operator unseal` rechaza shard**: posible drift entre instancia y
  shards. Solo el nuevo init genera nueva familia de shards. Si reinicializaron
  Vault, los shards anteriores no sirven — usa procedimiento de recovery total.
- **`vault status` no responde**: el contenedor no arrancó. `docker logs vault`
  para diagnóstico. Causas comunes: volumen sin permisos, `disable_mlock`
  desactivado en host sin `IPC_LOCK` capability.
