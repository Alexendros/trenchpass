# Runbook · Incident response (P0/P1)

> Cuándo: el dashboard SigNoz dispara una alerta P0/P1 o el operador detecta
> degradación del servicio. Para P2/P3, abre issue y reagenda.

## Severidades

| Sev | Significado | Tiempo de reacción |
|:---:|-------------|:------------------:|
| **P0** | gateway no responde / audit log se rompe / Vault corrupto | <15 min |
| **P1** | error rate >5 %, scope_violations sostenidas, replay floods | <60 min |
| **P2** | drift Proton Pass, alerta de heartbeat ManitasFritas | <24 h |
| **P3** | docs desactualizados, dashboards rotos | best-effort |

## Procedimiento (P0/P1)

1. **Acuse**: comenta en el ticket SigNoz con `acked by alexendros @ HH:MM`.
2. **Comunica** (si afecta consumidores): nota en Notion lobby Apsys con
   asunto `[P0] TrenchPass degraded — investigando`.
3. **Triaje** rápido (≤5 min):
   - `kubectl get pods` o `docker ps` (gateway running?).
   - `vault status` (sealed?).
   - `psql -c 'SELECT 1' postgres-audit` (alive?).
   - SigNoz: ¿qué namespace falla? ¿desde cuándo?
4. **Mitiga** primero, diagnostica después:
   - Si Vault sellado → `runbooks/vault-unseal.md`.
   - Si gateway crashloop → `docker logs trenchpass --tail 200 | head` y reinicia.
   - Si Postgres-audit lleno → expande disco o purga `audit_events` >180 d.
5. **Captura evidencia**:
   ```bash
   docker logs trenchpass --since 30m > /tmp/trenchpass-incident-$(date -Iseconds).log
   curl https://signoz/api/v1/queries/range?... > /tmp/signoz-incident.json
   ```
6. **Resuelve** y verifica con smoke (ver `docs/api.md`).
7. **Postmortem** dentro de 5 días hábiles si fue P0:
   - Plantilla en `docs/runbooks/postmortem-template.md`.
   - Acciones de mejora se convierten en issues etiquetadas `postmortem`.

## Comunicación

| Canal | Severidad | Audiencia |
|-------|:---------:|-----------|
| Notion lobby Apsys | P0/P1 | operadores y stakeholders |
| GitHub Issue | P0/P1/P2 | comunidad |
| Cuaderno SuperKrabo bitácora | todas | interno |
| Status page (PR9) | P0/P1 | público |

## Escalación

Para Alexendros operador único, escalación = pedir ayuda externa (consultor
Rust o asesor seguridad). En caso de incapacitación: **ManitasFritas** se
encarga al cabo de 90 d.
