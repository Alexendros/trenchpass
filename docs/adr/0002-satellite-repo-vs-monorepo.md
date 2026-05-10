# ADR-0002 · Repo satélite Cargo vs miembro Turborepo

- Fecha: 2026-04-28
- Estado: Aceptado
- Decisor: Alexendros

## Contexto

Controlink es un monorepo Turborepo + pnpm con `apps/*` y `packages/*`. La
tentación natural es añadir `apps/controlink-mcp/` como otro miembro. Pero
Turborepo no entiende Cargo: no comparte cache, no ejecuta `cargo build` con
sus heurísticas, no orquesta `cargo workspace` correctamente.

## Decisión

TrenchPass vive en **repo satélite independiente** `github.com/Alexendros/trenchpass`.
El contrato hacia Controlink son tres elementos que sí viven en el monorepo:

- `packages/mcp-client-ts/` — cliente TypeScript que usa el gateway.
- `packages/shared-types/openapi.yaml` — schema generado desde `cargo run -- --dump-schema`.
- `infra/` — configs de Vault, Postgres, SigNoz, Traefik (compartidas con otras apps).

## Consecuencias

- Versionado independiente: el gateway puede liberar `v0.3.0` sin afectar
  versionado de Controlink.
- Dos remotes a vigilar (`Controlink` + `trenchpass`).
- CI separado: GHA del gateway corre Rust, GHA de Controlink corre Node.
- Releases coordinadas vía `mcp-client-ts` versiones.
- Forgejo replica ambos.

## Alternativas descartadas

- **Cargo workspace dentro de Turborepo**: Turborepo no respeta `Cargo.lock`,
  invalida cache de manera errática, y obliga a hacks como `pnpm dlx cargo`.
- **Submódulo git en Controlink**: pesado para los desarrolladores TS;
  rompe `pnpm install` cuando el submódulo se desincroniza.
