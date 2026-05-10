# ADR-0001 · Rust como lenguaje del gateway

- Fecha: 2026-04-28
- Estado: Aceptado
- Decisor: Alexendros

## Contexto

El gateway custodia los secretos de toda la flota. Un fallo de seguridad o un
crash en el camino caliente afecta a todos los consumidores. Los candidatos
realistas eran:

- **Node/TypeScript** — homogéneo con Controlink y los consumidores.
- **Go** — el lenguaje de Vault, Pomerium, OpenBao.
- **Rust** — `rmcp` SDK oficial nativo, footprint mínimo, type safety.

## Decisión

Construir el gateway en **Rust** sobre `rmcp`, `axum`, `tokio`.

## Justificación

| Eje | TypeScript | Go | Rust |
|-----|:----------:|:--:|:----:|
| SDK MCP de primera clase | TS oficial | sin SDK oficial | **rmcp oficial** |
| Type safety en wire layer | parcial | parcial | **fuerte** |
| Footprint imagen | ~150 MB | ~80 MB | **~12 MB** |
| Latencia p99 (cache hit) | ~10 ms | ~3 ms | **<2 ms** |
| Memory safety sin GC | n/a | parcial | **total** |
| Ecosistema rustls/Vault/SigNoz | parcial | bueno | **bueno** |
| Coste de aprendizaje | bajo | medio | alto |

El coste de aprendizaje se asume con el manual de estilo + `clippy` agresivo.

## Consecuencias

- Toolchain Rust requerido en local + CI. Documentado en `CONTRIBUTING.md`.
- Compilación más lenta (Cargo full release ~3 min) compensada con cache GHA.
- Tests más estrictos, menos flakes en runtime.
- El operador debe mantener una toolbox Fedora con `cargo` instalado o usar el
  Dockerfile como dev environment.

## Alternativas descartadas

- **TS**: la falta de mTLS doble factor a nivel handler es resoluble con
  middleware, pero el footprint y la herencia de `process.env` lo descalifican.
- **Go**: razonable; perdimos el SDK oficial MCP y Rust gana en footprint y
  type safety. Si `rmcp` se abandonara, reconsideramos en ADR posterior.
