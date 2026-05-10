# Contribuir a TrenchPass

Gracias por considerar contribuir. Lee este documento entero antes de abrir
una issue o un PR.

## Antes de nada

- **Lee `CODE_OF_CONDUCT.md`**.
- **Lee `SECURITY.md`** si tu cambio toca auth, mTLS, scopes, vault, audit
  o cualquier capa defensiva. Reportes de vulnerabilidad por canal privado.
- **Conoce el contexto**: `ARCHITECTURE.md` describe el modelo mental;
  `ROADMAP.md` lista los PRs en orden; `docs/adr/` recoge decisiones cerradas.

## Configuración local

```bash
# Fedora Silverblue: usa toolbox
toolbox create --release 43 trenchpass-dev
toolbox enter trenchpass-dev
sudo dnf install cargo clippy rustfmt rust-src protobuf-compiler \
    openssl-devel pkgconf-pkg-config postgresql

# Pre-commit hooks
cargo install --locked cargo-deny cargo-audit cargo-about
```

Levanta el stack mínimo (Vault + Postgres-audit + otel-collector):

```bash
docker compose -f infra/dev/docker-compose.yml up -d
cp .env.example .env
# rellena VAULT_TOKEN y TRENCHPASS_DEV_BEARER
cargo run
```

## Flujo de trabajo

1. Crea una rama desde `main`: `feat/<scope>-<resumen-corto>` o `fix/...`.
2. Una sola idea por PR. Si tu cambio toca >5 archivos en módulos distintos,
   probablemente debería ser N PRs.
3. **Tests obligatorios** para todo código en `src/auth/`, `src/security/`,
   `src/audit/`. PR sin tests en estos módulos se rechaza sin revisar.
4. Ejecuta antes de pushear:
   ```bash
   cargo fmt --all
   cargo clippy --all-targets -- -D warnings
   cargo test --locked
   cargo deny check
   ```
5. Si añades una crate, justifica en el PR por qué (footprint, mantenimiento,
   licencia compatible).
6. Si tu cambio rompe un comportamiento documentado, actualiza
   `CHANGELOG.md` bajo `## [Unreleased]`.

## Estilo de commit

Convencional, en castellano (proyecto interno · español manda):

```
<tipo>(<scope>): <verbo en infinitivo>

[cuerpo opcional con justificación, contexto y trade-offs]

[footer opcional con `Refs:`, `Closes:`, `Co-authored-by:`]
```

`tipo` ∈ `feat | fix | refactor | docs | test | chore | perf | sec | infra | revert`.
`scope` ∈ módulo afectado (`auth`, `vault`, `audit`, `tools/notion`, `ci`…).

Ejemplo:

```
feat(auth/scope): añadir wildcard parcial `notion:read_*`

Permite scopes intermedios entre exact-match y namespace-wildcard, útil
para cuentas CI que solo deben leer.

Refs: #42
```

## Estilo de código

- `rustfmt` con `rustfmt.toml` versionado. No discutir estilo en PRs.
- `clippy` con `clippy.toml`. Si una lint molesta, abre un ADR.
- Prohibido `unwrap()` en código de producción salvo `OnceCell::get_or_init`.
- Prohibido `unsafe` salvo bloque comentado con justificación + invariantes.
- Prohibido `allow(dead_code)` salvo en stubs PR-N → PR-N+1 marcados con TODO.

Más detalles en `STYLEGUIDE.md`.

## Testing

| Capa | Herramienta | Cobertura mínima |
|------|-------------|-------------------|
| Unit (`#[cfg(test)]`) | `cargo test` | 80 % en `src/auth/`, `src/security/` |
| Property | `proptest` | scopes, replay, rate limit |
| Integration | `axum-test` | un caso por handler |
| End-to-end | `infra/tests/` (PR3+) | smoke tools + rotación |
| Fuzzing | `cargo-fuzz` (PR9) | parser PEM, scope JSON |

## Revisión

- Todo PR necesita 1 review. PRs que tocan auth, mTLS, scopes o audit necesitan 2.
- El reviewer comprueba: tests, clippy, deny check, ADR si aplica.
- El autor merge cuando la rama está verde y el reviewer ha aprobado.

## Reportar bugs

Usa la plantilla `.github/ISSUE_TEMPLATE/bug.md`. Incluye:

- Versión: `trenchpass --version` o git SHA del binario.
- Entorno: `TRENCHPASS_ENV`, versión Vault, Postgres, Traefik.
- Trace ID si está disponible (lo encuentras en SigNoz).
- Reproducción mínima.

## Reportar vulnerabilidades

**No** abras una issue pública. Lee `SECURITY.md`.
