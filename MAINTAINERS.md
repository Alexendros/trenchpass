# Maintainers

> Quién decide qué se mergea, quién hace los releases, quién responde a security advisories.

| Rol | Persona | Áreas | Backup |
|-----|---------|-------|--------|
| **Lead maintainer** | Alexendros (`spiderwebtraveler@proton.me`) | todo | — |
| **Security contact** | Alexendros | `src/auth/`, `src/security/`, `src/vault/`, `infra/` | — |
| **Release manager** | Alexendros | `Cargo.toml`, `CHANGELOG.md`, tags | — |
| **Operator on-call** | Alexendros | gateway en producción | — |

## Onboarding nuevo maintainer (procedimiento)

1. Tres meses de contribución sostenida con quality bar consistente.
2. Lead maintainer abre RFC bajo issue.
3. Si no hay objeción en 14 días, se otorga rol con scope inicial limitado:
   - Triage de issues.
   - Review de PRs (sin merge directo).
4. Tras 6 meses adicionales, se considera elevación a merge rights por área concreta.

## Privilegios por rol

| Rol | Triage | Review | Merge | Tag release | Cosign sign | Manage members |
|-----|:------:|:------:|:-----:|:-----------:|:-----------:|:--------------:|
| Lead | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| Maintainer | ✅ | ✅ | área asignada | ❌ | ❌ | ❌ |
| Triager | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ |

## Sucesión

En caso de incapacitación del lead maintainer, ManitasFritas (90 días sin
heartbeat) activa:

1. Distribución de instrucciones a recipientes designados.
2. Reconstrucción Shamir 3-de-5 de la unseal Vault.
3. Promoción del recipiente con mayor experiencia técnica a lead temporal
   hasta convocatoria de elección entre maintainers activos.

Los detalles del procedimiento (recipientes, k-de-n, instrucciones) viven
fuera del repo, en sobres físicos sellados y bóvedas Proton compartidas.

## Conflicto de interés

Si un maintainer está empleado por Alexendros o por una empresa cliente que
consume TrenchPass, debe declararlo en este archivo bajo `## Disclosure` y
abstenerse de votos sobre features que afecten directamente a su empleador.

## Disclosure

_Sin conflictos al cierre de PR1._
