# Runbook · Drill semestral de recovery

> Cuándo: cada 6 meses (1 enero, 1 julio) o tras cualquier cambio mayor en
> la topología de custodia.

## Objetivos

1. Confirmar que **3 de los 5 shards Shamir** son legibles y reúnen la unseal key.
2. Confirmar que **Proton Pass espejo** sigue sincronizado y se puede exportar.
3. Confirmar que el **backup R2** del audit log se descifra correctamente.
4. Cronometrar el procedimiento end-to-end y compararlo con el drill anterior.

## Pre-drill (T-7 días)

- [ ] Avisar a recipientes de shards físicos (no requieren venir, solo confirmar acceso).
- [ ] Comprobar `gpg --list-keys` del operador y de cada custodio digital.
- [ ] Generar entorno aislado (toolbox `trenchpass-drill`).
- [ ] Reservar 4 h de calendario.

## Procedimiento

### Fase 1 · Recuperación de shards (60 min)

```bash
# En entorno aislado, sin red al Vault productivo.
docker run --rm -it -v drill-vault:/vault/data hashicorp/vault:latest server -config=/vault/config/vault.hcl

# En otro terminal:
export VAULT_ADDR=http://localhost:8200
vault operator init -key-shares=5 -key-threshold=3
# Anota los nuevos shards drill (no usar fuera del entorno).

# Para validar shards productivos: clónalos desde sus ubicaciones (Proton, USB,
# papel, custodio externo, R2 cifrado) y confirma que reúnen 3 de 5 con un
# Vault en modo "test-recovery":
vault operator unseal <shard_1>
vault operator unseal <shard_2>
vault operator unseal <shard_3>
```

### Fase 2 · Export Proton Pass (45 min)

1. En toolbox aislada, instalar `protonpass-cli`.
2. Login con biometric en cuenta separada del operador (alias `drill@proton.me`).
3. Importar bóveda exportada a JSON.
4. Comparar contra snapshot de Vault drill: cada `secret/providers/<x>` debe
   tener su entrada en Proton Pass con `version` igual.

### Fase 3 · Restore audit (30 min)

```bash
# Descargar último backup R2.
rclone copy r2:trenchpass-backups/audit/<latest>.sql.gpg /tmp/

# Descifrar con clave operador.
gpg --decrypt /tmp/<latest>.sql.gpg > /tmp/audit-drill.sql

# Restaurar a Postgres aislado.
psql -h localhost -U postgres -d audit-drill -f /tmp/audit-drill.sql

# Smoke: contar eventos de los últimos 7 d.
psql -c "SELECT count(*) FROM audit_events WHERE ts > NOW() - INTERVAL '7 days';"
```

### Fase 4 · Métricas y cierre (45 min)

- [ ] Cronómetro total de las fases 1–3.
- [ ] Anota en `docs/runbooks/drill-history.md` (gitignored si PII).
- [ ] Si alguna fase superó 1 h, abrir issue `recovery-too-slow`.
- [ ] Anuncia en cuaderno SuperKrabo `bitacora.md` con marca `drill OK YYYY-MM-DD`.

## Criterios de éxito

| Métrica | Objetivo |
|---------|---------|
| Tiempo total | ≤4 h |
| Shards recuperados | 3 de 5 mínimo |
| Drift Vault↔Proton | 0 entries |
| Audit restore | 100 % filas comparable a snapshot pre-backup |

## Acciones si falla

- **Shard ilegible** → re-emitir 5 shards nuevos y reagendar drill +30 d.
- **Proton Pass desincronizado** → forzar sync worker manual y diagnose drift.
- **Backup R2 corrupto** → verificar política de rclone y configurar tripletas
  de backup (R2 + Hetzner + offline).
