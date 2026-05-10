# Release process

> Cada release es un commit firmado en `main`, una tag SemVer, una imagen
> Docker firmada con `cosign` y una entrada en `CHANGELOG.md`.

## SemVer aplicado

| Salto | Cuándo |
|-------|--------|
| **MAJOR** | Cambio incompatible en la wire API MCP (nombres de tool, schema, scopes). |
| **MINOR** | Nueva tool, nuevo namespace, nuevo header opcional, nueva env var no obligatoria. |
| **PATCH** | Fix de bug, hardening interno, mejora de observabilidad sin cambio de contrato. |

Pre-1.0 (`0.x.y`): los saltos MINOR pueden romper el wire — documentados en CHANGELOG con etiqueta **BREAKING**.

## Pasos del release

1. **Cherry-pick a `main`** (o merge tras pasar CI).
2. **Actualiza** `CHANGELOG.md` moviendo `## [Unreleased]` a `## [X.Y.Z] · YYYY-MM-DD · <slug>`.
3. **Bump** `Cargo.toml` `version` y `CITATION.cff` `version` + `date-released`.
4. **Commit firmado**: `git commit -S -m "release: vX.Y.Z"`.
5. **Tag firmada**: `git tag -s vX.Y.Z -m "release vX.Y.Z"`.
6. **Push**: `git push origin main vX.Y.Z`.
7. CI publica imagen `ghcr.io/alexendros/trenchpass:X.Y.Z` y la firma con cosign.
8. **GitHub release**: notas extraídas del CHANGELOG; adjunta `THIRD_PARTY_NOTICES.md` regenerado.
9. **Anuncio**: comentario en Notion lobby Apsys + push del cuaderno SuperKrabo.

## Firma de imágenes (cosign)

```bash
cosign sign \
  --key cosign.key \
  ghcr.io/alexendros/trenchpass:X.Y.Z

cosign verify \
  --key cosign.pub \
  ghcr.io/alexendros/trenchpass:X.Y.Z
```

La clave pública de firma vive en `docs/cosign/operator.pub` y se publica
también en el `keys.openpgp.org` del operador.

## Hotfix

1. Rama `hotfix/vX.Y.Z+1` desde la tag `vX.Y.Z`.
2. Fix mínimo + test que reproduzca.
3. PR contra `main`. Si la rama estable diverge, también contra `release/X.Y`.
4. Sigue el proceso normal de release.

## Yanking

Si un release introduce una vulnerabilidad seria:

```bash
# crates.io no aplica — no publicamos a crates.io.
# Para imagen Docker:
gh release edit vX.Y.Z --draft=true --notes "Yanked: ver advisory GHSA-…"
# Mantener imagen pero documentar en CHANGELOG y advisory.
```

## Compromiso de soporte

- **Latest minor**: bug fixes y security fixes.
- **Previous minor**: solo security fixes durante 90 días tras el release del siguiente minor.
- **Versiones más antiguas**: best-effort, sin garantía.

## Calendario tentativo

- **Cadencia normal**: una minor cada ~6 semanas.
- **Cadencia post-incidente**: hotfix dentro de 48 h del descubrimiento.
- **Releases mayores**: solo cuando un cuaderno propuesta lo justifique.
