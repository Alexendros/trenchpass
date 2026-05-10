# Postmortem template

> Rellena este archivo bajo `docs/postmortems/YYYY-MM-DD-<slug>.md` tras un
> incidente P0 (o P1 si fue público). No es ejercicio de buscar culpable;
> es ejercicio de mejorar el sistema.

## Resumen

- **Fecha**: YYYY-MM-DD HH:MM Europe/Madrid
- **Severidad**: P0 / P1
- **Duración del impacto**: <X min>
- **Servicios afectados**: <gateway / vault / audit / consumidores>
- **Detección**: <SigNoz alert / consumer reportó / monitoring externo>
- **Resolución**: <quién aplicó qué fix>

## Línea de tiempo (UTC)

| Hora | Evento |
|------|--------|
| HH:MM | Trigger inicial |
| HH:MM | Primera alerta |
| HH:MM | Acuse del operador |
| HH:MM | Mitigación aplicada |
| HH:MM | Servicio restaurado |
| HH:MM | Resuelto |

## Impacto

- Consumers afectados: <lista>
- Requests fallidas: <count>
- Datos perdidos: <sí/no, qué>
- Audit log degradado: <sí/no>
- Cumplimiento SLA / contractual: <impacto>

## Causa raíz

<Descripción técnica honesta. Distingue **causa proximal** y **causa raíz**.>

## Lo que funcionó

- ...
- ...

## Lo que no funcionó

- ...
- ...

## Acciones de mejora

| # | Acción | Owner | Issue | Plazo |
|---|--------|-------|-------|-------|
| 1 |        |       |       |       |
| 2 |        |       |       |       |

## Lecciones para el modelo de amenazas

<¿Hay un nuevo vector que añadir a `docs/threat-model.md`? ¿Una asunción que cambió?>

## Anexos

- Logs: <ruta>
- Trace SigNoz: <URL>
- Diff aplicado: <commit hash>
