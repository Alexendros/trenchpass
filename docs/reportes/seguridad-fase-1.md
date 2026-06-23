# Informe · Fixes de seguridad P1 · TrenchPass

**Rama:** `fix/seguridad-fase-1` (base `origin/main`)
**Commit:** HEAD de la rama `fix/seguridad-fase-1` — _seguridad: fixes P1 en workflows, deps y runtime_
**Push:** No realizado, conforme a instrucciones.

## Hallazgos cerrados

| Severidad | Hallazgo                                                    | Archivo(s) / líneas                                                            | Fix aplicado                                                                                                                                           |
| --------- | ----------------------------------------------------------- | ------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------ |
| ALTO      | `quinn-proto` 0.11.14 (`RUSTSEC-2026-0185`)                 | `Cargo.lock` L2727                                                             | `cargo update -p quinn-proto` → `0.11.15`. Verificado con `cargo audit --json` (0 vulnerabilidades).                                                   |
| ALTO      | `pull_request_target` + filtro actor inseguro en auto-merge | `.github/workflows/dependabot-automerge.yml` L21, L30, L33-34                  | Cambiado a `pull_request`; validación por `github.event.pull_request.user.login == 'dependabot[bot]'` y `user.type == 'Bot'`; permisos mínimos al job. |
| ALTO      | Permisos de workflow-scope en release                       | `.github/workflows/release.yml` L9, L30-33                                     | Workflow con `contents: read`; `contents: write`, `packages: write`, `id-token: write` sólo en `build-and-publish`.                                    |
| ALTO      | 43 acciones sin pin SHA                                     | Todos los workflows                                                            | Todas las acciones pinnadas a SHA con comentario de versión (ej. `actions/checkout@df4cb1c... # v6`).                                                  |
| MEDIO     | `ci.yml` sin permissions                                    | `.github/workflows/ci.yml` L9, L16-17, L25-26, L37-38, L51-53                  | `permissions: contents: read` a nivel workflow; permisos mínimos por job (`contents: read`; `packages: write` para docker).                            |
| MEDIO     | `security-events: write` a nivel workflow                   | `.github/workflows/security.yml` L9, L48-49, L68-69                            | `security-events: write` movido a `codeql` y `trivy` (jobs que suben SARIF).                                                                           |
| MEDIO     | Fallback a `X-Forwarded-Tls-Client-Cert` sin validar proxy  | `src/auth/mtls.rs` L36-45, L61-68; `src/security/middleware.rs` L75-81, L94-95 | `extract_cn` ahora recibe `tls_mode` y `header_trusted`; rechaza el header si `tls_mode == Off` y no se declaró `TRENCHPASS_MTLS_HEADER_TRUSTED`.      |
| MEDIO     | Default `TRENCHPASS_TLS_MODE=off` en producción             | `src/config.rs` L67, L170, L193-198                                            | Nuevo campo `mtls_header_trusted`; en `Environment::Production` se exige `static` o `vault_pki` y se devuelve error.                                   |
| MEDIO     | `init_crypto` warn en vez de fatal                          | `src/lib.rs` L27-46; `src/main.rs` L33-36                                      | `init_crypto` devuelve `bool`; en producción un provider distinto a `aws-lc-rs` produce `bail` fatal.                                                  |
| MEDIO     | Descarga de `cargo-about` desde `releases/latest`           | `.github/workflows/release.yml` L74-85                                         | Pinned a `0.9.0` y verificación SHA-256 (`7a1a1bfb3ae3b6feabe9e1d6147a6b34b85bd0339bfd8b0ec11b312dee10d99a`).                                          |
| BAJO      | `actions/checkout` sin `persist-credentials: false`         | Todos los workflows                                                            | Añadido a los 9 checkout existentes.                                                                                                                   |
| BAJO      | Enlaces rotos en docs                                       | `SUPPORT.md` L11-12, `ARCHITECTURE.md` L77, `CHANGELOG.md` L78                 | Templates `.md` → `.yml`; `sql/init_audit.sql` → `migrations/20260510120000_init_audit.sql`.                                                           |

## Archivos cambiados

```text
 .github/workflows/ci.yml
 .github/workflows/dependabot-automerge.yml
 .github/workflows/release.yml
 .github/workflows/security.yml
 ARCHITECTURE.md
 CHANGELOG.md
 Cargo.lock
 SUPPORT.md
 docs/reportes/seguridad-fase-1.md
 examples/refresh_smoke.rs
 src/auth/mtls.rs
 src/config.rs
 src/lib.rs
 src/main.rs
 src/security/middleware.rs
```

## Verificaciones ejecutadas

| Comando                                              | Resultado                                    |
| ---------------------------------------------------- | -------------------------------------------- |
| `cargo check --locked --all-targets`                 | OK                                           |
| `cargo clippy --all-targets --locked -- -D warnings` | OK                                           |
| `cargo test --locked --all-targets`                  | OK — 96 tests pasados                        |
| `cargo audit --json`                                 | OK — `vulnerabilities.found: false`, count 0 |
| `cargo fmt --all --check`                            | OK                                           |
| `git diff --check`                                   | Sin errores de espacio                       |

## Notas y pendientes

- No se ha hecho `push`. La rama está lista para revisión/PR.
- `cargo audit` mantiene los ignores preexistentes `RUSTSEC-2023-0071` y `RUSTSEC-2025-0134` (configurados en `deny.toml`). Revisar si es posible cerrarlos en una fase posterior.
- Los SHAs pinnados corresponden a las versiones activas al 2026-06-23; si se actualizan las actions, conviene revisar el pinning periódicamente.
- El campo `TRENCHPASS_MTLS_HEADER_TRUSTED` debe usarse con cautela: sólo activarlo si el proxy upstream termina mTLS y no puede inyectar `PeerCertificate` directamente.
