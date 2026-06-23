# Soporte

TrenchPass es **software de un solo operador** mantenido por Alexendros.
No hay SLA público ni soporte comercial. Lee este documento antes de abrir
una issue.

## ¿A dónde voy con mi pregunta?

| Pregunta | Canal |
|----------|-------|
| Bug | [GitHub Issues · template `bug`](https://github.com/Alexendros/trenchpass/issues/new?template=bug.yml) |
| Feature request | [GitHub Issues · template `feature`](https://github.com/Alexendros/trenchpass/issues/new?template=feature.yml) |
| Pregunta de uso | [GitHub Discussions](https://github.com/Alexendros/trenchpass/discussions) |
| Pregunta de arquitectura | Lee `ARCHITECTURE.md` y `docs/adr/`. Si sigue sin estar claro, abre Discussion. |
| Vulnerabilidad | **NO uses Issues** · `SECURITY.md` |
| Soporte de operación (Alexendros interno) | Cuaderno `agente__SuperKrabo_Controlink-MCP-arquitecto-gateway-vault-mtls/bitacora.md` |

## Antes de abrir una issue

- ¿Estás en la última versión? `trenchpass --version` o `git log --oneline -1`.
- ¿Has leído `CHANGELOG.md`?
- ¿Has buscado issues abiertas y cerradas con palabras clave?
- ¿Tienes el `trace_id` desde SigNoz si la issue es de runtime?

## Lo que va a pasar tras abrir la issue

1. Triaje en ≤ 7 días: etiqueta + prioridad (`P0` blocker, `P1` alto, `P2` normal, `P3` nice-to-have).
2. Si es bug reproducible: candidato al siguiente sprint.
3. Si es feature: depende del roadmap. Las que no encajen quedan en `Wishlist` con etiqueta de cuándo se reconsideran.

## Garantías explícitas

- **Ninguna**. AGPL §15 + §16 aplican.
- Best-effort en bug fixes que afecten a la flota Alexendros.
- Soporte cero a forks salvo que paguen consultoría.

## Consultoría

Si necesitas desplegar TrenchPass en tu propia flota y quieres ayuda directa:
contacta `spiderwebtraveler@proton.me` con asunto `[TrenchPass-consult]`.
