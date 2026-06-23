---
name: ecosystem-conventions
description: >
  Convenciones del ecosistema Alexendros y guía de desarrollo específica para
  trenchpass (gateway MCP de credenciales en Rust). Activar al trabajar en
  este repo o al necesitar contexto sobre patrones Rust y seguridad del ecosistema.
---

# Convenciones Ecosistema Alexendros — TrenchPass

## Repo Profile
- **Tipo**: rust-mcp (credential gateway)
- **Stack**: Rust 1.88+, Axum, HashiCorp Vault, mTLS, aws-lc-rs
- **Licencia**: AGPL-3.0 (obligatorio: enlaza sequoia-openpgp copyleft)
- **Deploy**: Binario Rust con servidor mTLS

## Comandos esenciales
```bash
cargo build                                    # compilar
cargo build --release                          # compilar release
cargo clippy --all-targets -- -D warnings      # lint
cargo test                                     # tests
cargo audit                                    # auditoría de vulnerabilidades
cargo fmt --check                              # verificar formato
```

## Pre-PR Checklist
1. `cargo fmt --check` — formato correcto
2. `cargo clippy --all-targets -- -D warnings` — sin warnings
3. `cargo test` — todos los tests pasan
4. `cargo build --release` — compila sin errores
5. `cargo audit` — sin vulnerabilidades críticas

## Reglas de código
- Rust edition 2021, rust-version 1.88+
- AGPL-3.0: cada archivo fuente debe mantener header de licencia si existe
- aws-lc-rs para FIPS compliance
- Certificados mTLS cortos (<=7d) vía Vault PKI
- NUNCA loguear secrets, tokens ni credenciales
- NUNCA almacenar credenciales en texto plano fuera de Vault

## Git
- Branch: `devin/<timestamp>-<descripcion>`
- Commits: Conventional Commits
- Nunca push directo a `main`

## Secrets requeridos
- `VAULT_ADDR` — dirección del servidor Vault
- `VAULT_TOKEN` — token de acceso (PKI + KV)
- `DATABASE_URL` — PostgreSQL para auditoría
- mTLS: certificados auto-generados vía Vault PKI
- Ubicación en Devin: `/run/repo_secrets/trenchpass/.env.secrets`

## Seguridad
- Este es un GATEWAY DE CREDENCIALES — seguridad máxima
- Ejecución triphasic: dry-run → sandbox → real
- NUNCA ejecutar acciones destructivas sin confirmación
- NUNCA exponer endpoints sin mTLS
- Audit log obligatorio para toda operación con credenciales

## Anti-patrones
- NO usar `unwrap()` en código de producción (usar `?` o `expect` con mensaje)
- NO usar `unsafe` sin justificación documentada
- NO desactivar clippy lints sin comentario explicativo
- NO hacer cargo update sin verificar changelog de crates actualizados
