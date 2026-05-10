# Style guide · Rust

> Hace cumplir `rustfmt.toml` + `clippy.toml`. Cualquier discusión de estilo
> que no esté aquí, se resuelve en un ADR.

## Idioma

- Comentarios y docstrings en castellano. Nombres de identificadores en inglés.
- README, CHANGELOG, ARCHITECTURE en castellano (proyecto interno).
- Mensajes de log y trazas (campos `target`, `event`) en inglés (consumidos por SigNoz).

## Naming

| Item | Convención |
|------|------------|
| Crates | `kebab-case` (`trenchpass`) |
| Módulos | `snake_case` (`gocardless_dd`) |
| Tipos / structs / enums / traits | `PascalCase` (`AuditStore`) |
| Funciones / variables | `snake_case` |
| Constantes | `SCREAMING_SNAKE_CASE` |
| Variables de entorno | `TRENCHPASS_<SECCION>_<CAMPO>` o convención del SDK (`VAULT_ADDR`, `OTEL_EXPORTER_OTLP_ENDPOINT`). |

## Errores

- Un `enum Error` por crate (en `src/error.rs`).
- `thiserror::Error` para tipos finales; `anyhow::Result` solo en main/binarios.
- Cada handler axum devuelve `Result<T, Error>`; nunca `Result<T, anyhow::Error>` (rompe `IntoResponse`).
- `unwrap()` y `expect()` prohibidos salvo:
  - Inicialización de `OnceCell`/`LazyLock`.
  - Tests (`#[cfg(test)]`).
  - `NonZero*::new(...).expect("n > 0")` cuando la entrada es estática.
- `panic!()` solo si el invariante violado significa pérdida de seguridad
  (ej. clave Vault corrupta tras parse).

## Async

- `#[tokio::main]` solo en `main.rs`.
- Spawning con `tokio::spawn` debe propagar errores: usa `tracing::error!` + métrica antes de salir.
- Long-running tasks van en `tokio::task::JoinSet` con `JoinSet::join_next`.
- Nunca `block_on` dentro de async; nunca `tokio::runtime::Runtime::new()` en código de runtime.

## Tracing

- Usa `#[instrument(skip(self), fields(...))]` en métodos públicos de capas core
  (vault, audit, security).
- `target` convencional: `trenchpass.<modulo>` para logs internos,
  `<provider>.api` para llamadas externas (las recoge SigNoz como spans).
- Nivel:
  - `error!` → algo se ha roto, hace falta intervención.
  - `warn!`  → algo sospechoso pero recuperable (rate limit hit, replay rechazado).
  - `info!`  → eventos de ciclo de vida (boot, rotation, shutdown).
  - `debug!` → trazas del hot path.
  - `trace!` → desactivado por defecto en prod.

## Imports

- Orden: `std` → `third-party` → `crate::*`.
- Sin `use foo::*` salvo en preludes oficiales (`use rmcp::prelude::*;`).
- `rustfmt` agrupa/ordena automáticamente.

## Tests

- Unit tests en el mismo archivo bajo `#[cfg(test)] mod tests { … }`.
- Integration tests en `tests/<area>.rs` — uno por capa (auth, vault, audit, e2e).
- Nombres descriptivos: `fn rejects_other_namespace()` mejor que `fn test_scope_2()`.
- `proptest` para invariantes (rate limit, replay, scopes).
- No aceptamos tests que dependan de servicios externos sin `wiremock`.

## Comentarios

- Default: **no** comentes. Si un identificador se llama bien, no hace falta.
- Comentarios de **por qué**, no de **qué**:
  - ✅ `// `audit_writer` solo tiene INSERT → no usamos RETURNING.`
  - ❌ `// inserta el evento en la tabla`
- TODOs siempre con autor y tracking ID:
  - `// TODO(alex, PR3): activar mTLS estricto.`

## Dependencias

- Antes de añadir una crate: justifica en el PR (footprint, mantenimiento, licencia).
- Licencia compatible AGPL-3.0:
  - ✅ Apache-2.0, MIT, ISC, BSD-2/3, MPL-2.0, AGPL.
  - ❌ GPL-2.0-only sin upgrade clause; cualquier propietaria.
- Versiones pinneadas con `cargo update -p <crate>` solo bajo PR explícito.
- `cargo deny` falla el CI si la advisory de `cargo audit` afecta a una crate productiva.

## Comments en HCL/YAML/SQL

- Cabeceras de archivo siempre con propósito + dueño:
  ```hcl
  # Vault OSS · configuración de servidor.
  # Propósito: custodio primario de secretos para MCP Gateway Rust.
  ```

## Markdown

- Una `#` por archivo (título). Secciones empiezan en `##`.
- Listas con `-`, no `*`.
- Bloques de código con lenguaje declarado (` ```rust`, ` ```bash`).
- Tablas con cabecera + alineación cuando ayuda (números a la derecha).
