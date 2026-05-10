# ADR-0005 · Postgres dedicado para audit append-only

- Fecha: 2026-04-30
- Estado: Aceptado
- Decisor: Alexendros

## Contexto

¿Dónde escribimos el audit log? Tres opciones:

1. SQLite local del gateway (simple, frágil).
2. Postgres **compartido** con Controlink (`controlink_db`).
3. Postgres **dedicado** `postgres-audit` que solo conoce el gateway.

## Decisión

**Postgres dedicado** con dos roles:

- `audit_writer` (LOGIN, solo `INSERT` + `USAGE` sobre la secuencia).
- `audit_reader` (LOGIN, solo `SELECT` para el dashboard Controlink).

Schema versionado en `sql/init_audit.sql`.

## Consecuencias

- Aislamiento total entre runtime de Controlink y audit del gateway. Si
  Controlink comparte una instancia comprometida, el audit log no se altera.
- Backups independientes (cada hora a R2 cifrado).
- `RETURNING id` **prohibido** (requiere SELECT). El gateway no necesita el id
  para nada operacional; el dashboard lo lee con `audit_reader`.
- Doble pool (`controlink_db` + `postgres-audit`) en infra. Costo bajo: ambos
  en el mismo VPS.

## Alternativas descartadas

- **SQLite**: lock-by-default, sin replicación, hostil para queries SigNoz.
- **Postgres compartido con Controlink**: pone el audit a merced del bus de
  Controlink. Si la app rota una credencial PG y olvida actualizar la del
  gateway, perdemos audit silencioso.
- **ClickHouse de SigNoz como destino primario**: SigNoz es para observabilidad
  de runtime, no para audit con retención legal. Mantener separados.
