<!--
Recordatorios:
- Una idea por PR. Si tocas >5 archivos en módulos distintos, probablemente debería ser N PRs.
- Si tu PR toca auth/security/vault/audit, espera 2 reviews (CODEOWNERS lo aplica).
- Si añades una crate, justifica en el cuerpo (footprint, mantenimiento, licencia).
- Actualiza CHANGELOG.md bajo `## [Unreleased]` si rompes contrato o añades feature.
-->

## Resumen

<!-- 1-3 frases. Qué cambia y POR QUÉ. -->

## Tipo

- [ ] feat · nueva funcionalidad
- [ ] fix · bug fix (no breaking)
- [ ] perf · mejora de rendimiento sin cambio de contrato
- [ ] sec · hardening / fix de seguridad
- [ ] refactor · reorganización sin cambio de comportamiento
- [ ] docs · solo documentación
- [ ] test · solo tests
- [ ] chore / infra · CI, deps, scaffolding
- [ ] BREAKING · rompe wire API o `Cargo.toml` MSRV

## Cambios principales

<!-- Bulleted. Cada bullet referencia un archivo o módulo. -->

-

## Verificación

- [ ] `cargo fmt --all --check`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test --locked`
- [ ] `cargo deny check`
- [ ] CHANGELOG actualizado (si aplica)
- [ ] ADR añadido (si decisión arquitectónica)

## Riesgos / rollback

<!-- ¿Qué se rompe si esto sale mal? ¿Cómo se revierte? -->

## Refs

<!-- Closes #N · Refs: cuaderno SuperKrabo · Refs: propuesta_#95 -->
