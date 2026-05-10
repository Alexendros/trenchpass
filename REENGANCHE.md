# Re-enganche · TrenchPass MCP gateway · 2026-05-10 (final)

> Snapshot para reanudar trabajo en hilo nuevo. Versión post sesión maratón
> que completó PR3 + PR4 + PR5 + 5 sub-hardening PRs. Borrar tras leer si
> el siguiente hilo arranca con PR6.

## Estado main (commit `5a81f20`)

- **PR1** ✅ scaffold
- **PR2** ✅ tools notion/stripe/github + audit Postgres (PR2a #13 · PR2b #14 · PR2c #16)
- **PR3** ✅ mTLS rustls 0.23 (aws-lc-rs) + Vault PKI + refresh loop (#17)
  - **PR3.1** ✅ hardening: trust anchors siempre refrescados, retry backoff 30s, otel::shutdown RAII (#18)
  - **PR3.2** ✅ peer-cert wiring TLS→auth via custom Accept (#19)
  - **PR3.3** ✅ OTel migration 0.26→0.31 builder API (#20)
- **PR4** ✅ sync worker drift detection (Manifest YAML ↔ Vault KV v2) (#21)
- **PR5** ✅ 10 namespaces restantes (#22)
  - **PR3.4** ✅ PR5 hardening: empty-string guards, AuthScheme enum (B2), tests por ns (#23)
  - **PR3.5** ✅ test fidelity: extracción guards, PartialEq, named span retro-compat (#24)

**Cero PRs abiertas** · **70/70 tests verde** · **clippy + fmt limpios**.

## Configuración GitHub aplicada

- Branch protection en `main`:
  - PR required, approvals=0
  - Status checks strict (fmt · clippy · test · cargo-audit · cargo-deny · CodeQL · Trivy · GitGuardian)
  - Required signatures (GPG `4737DF63620BFA61`)
  - Linear history · no force-push · no deletion · enforce_admins=false (bypass admin para emergencias)
- `allow_auto_merge=true` a nivel repo
- `delete-branch-on-merge=true` · squash merge único

## Componentes vivos (módulos `src/`)

```
src/
├── main.rs            · OtelShutdownGuard RAII · custom acceptor (PeerCertAcceptor → RustlsAcceptor)
├── lib.rs             · init_crypto() OnceLock con warn-on-Err
├── config.rs          · TlsMode {Off,Static,VaultPki} + ProtonPass{manifest_path}
├── audit/             · Postgres append-only (record + record_best_effort)
├── auth/
│   ├── mtls.rs        · extract_cn dispatcher (PeerCertificate ext → header Traefik)
│   └── ...
├── otel/setup.rs      · SdkTracerProvider 0.31 builder pattern · Resource::builder
├── security/          · rate_limit, replay, middleware
├── sync/              · manifest YAML, ManifestSource, detect_drift, spawn_drift_worker
├── tools/             · 13 namespaces (notion, stripe, github, forgejo, dokploy,
│                        hostinger, vercel, n8n, glitchtip, docuseal,
│                        gocardless_dd, gocardless_psd2, proton)
│   ├── shared.rs      · AuthScheme enum {Bearer | Header} · auth_get_json + bearer_get_json wrapper
│   └── BaseUrls       · 12 URLs · production() + with_bases() para tests
├── transport/
│   ├── mtls.rs        · build → TlsHandle{config, initial_bundle} · spawn_refresh_loop
│   ├── peer_cert.rs   · PeerCertAcceptor + AddPeerCertService (tower Service)
│   └── sse.rs         · router base
└── vault/
    ├── client.rs      · cache DashMap + list_kv_paths recursivo
    └── pki.rs         · issue_cert + pki_ca_chain (cert::read serial="ca_chain")
```

## Examples (smokes manuales)

- `examples/refresh_smoke.rs` + `refresh_smoke_setup.sh` · Vault PKI cert refresh + CA rotate
- `examples/drift_smoke.rs` · sync drift detection 4 casos
- `examples/otel_smoke.rs` · OTLP gRPC pipeline (verificado contra otelcol-contrib 0.117)

## Lecciones operacionales aprendidas

1. **Push directo a `main` PROHIBIDO** (branch protection lo bloquea ahora).
2. **Cadena de PRs**: si B se basa en A, mergear A primero, rebasar B contra main, luego mergear B (lección de #15).
3. **Auto-merge habilitado** · usar `gh pr merge <N> --auto --squash --delete-branch` para PRs con CI verde.
4. **Hook `pre-destructive-guard`** bloquea `git branch -D` · usar `-d` (sólo merged) o trabajar dentro de la convención.
5. **Vault dev cae** y reqwest pool stale es un artifact de test (no production concern).
6. **rustls 0.23 NO tiene feature `tls13`** (siempre activo) · sólo `tls12` opt-in.
7. **vaultrs 0.8 NO tiene `cert::ca::chain`** · usar `cert::read(mount, "ca_chain")`.
8. **axum-server #162** (peer cert no expuesto) lo resolvemos con `PeerCertAcceptor` custom.

## Roadmap pendiente (post-sesión)

- **PR6** Controlink monorepo cutover (`packages/mcp-client-ts` + 16 fetchers + 10 scripts) · 4h downtime planificado sábado
- **PR7** vía-fax IMAP Proton + verificación PGP
- **PR8** ManitasFritas (heartbeat 30/60/90 + Shamir 3-de-5)
- **PR9** cleanup AGPL + retirar Sentry/GlitchTip si SigNoz cubre

## Próximo turno (sugerencia primer prompt nuevo hilo)

```
# Objetivo
Cerrar PR6 (cutover monorepo Controlink) o arrancar PR7 (vía-fax IMAP+PGP).

# Contexto
Lee REENGANCHE.md. Estado main = 5a81f20 (PR1-PR5 + 5 sub-hardening merged).
Cero PRs abiertas. 70/70 tests verde. Branch protection + GPG signing activo.
Auto-merge habilitado.

# Salida esperada
Tu elección entre PR6 vs PR7 + plan de ejecución.
```
