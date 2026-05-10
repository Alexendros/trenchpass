# Modelo de amenazas · TrenchPass

> Versión PR1. Se ampliará con datos reales de tráfico tras PR3.
> Marco: STRIDE para vectores; LINDDUN para privacidad.

## Activos

| Activo | Confidencialidad | Integridad | Disponibilidad |
|--------|:---:|:---:|:---:|
| Secretos de proveedor en Vault KV v2 | Crítica | Crítica | Alta |
| Material PKI (CA root + intermediates) | **Crítica** | **Crítica** | Alta |
| Audit log Postgres | Alta | **Crítica** (append-only) | Media |
| Bóveda Proton Pass espejo | Alta | Alta | Media |
| Shards Shamir 3-de-5 distribuidos | **Crítica** (cada uno individualmente) | **Crítica** | Alta |
| Tokens Bearer de consumidores | Alta | Alta | Alta |
| Trace IDs y métricas SigNoz | Baja | Media | Media |

## Adversarios

| ID | Adversario | Capacidad asumida |
|----|-----------|--------------------|
| A1 | Atacante remoto sin credenciales | red abierta · scan masivo |
| A2 | Atacante con consumer comprometido (Bearer válido) | invocar tools del scope |
| A3 | Atacante con root local en VPS Dokploy | leer FS del contenedor |
| A4 | Atacante con acceso al host de Vault | Vault sellado o `vault read` |
| A5 | Atacante de cadena de suministro (crate maliciosa) | RCE en build CI |
| A6 | Insider (mantenedor adicional con merge rights) | propone PR malicioso |
| A7 | Atacante temporal (5 min de acceso a YubiKey) | firma vía-fax falsa |
| A8 | Estado-nación con sub-poena a Proton AG | acceso a bóveda Proton Pass |

## STRIDE

| Vector | Activo | Amenaza | Control |
|--------|--------|---------|---------|
| **S**poofing | identidad consumer | A1 reusa Bearer filtrado | mTLS · cert CN match |
| Spoofing | comando vía-fax | A7 envía mail falso | PGP firma + TOTP + nonce + skew 5 min |
| **T**ampering | audit log | A4 modifica filas | rol `audit_writer` solo INSERT · backup R2 |
| Tampering | secreto Vault | A3 escribe sin desellar | Vault sellado por defecto |
| **R**epudiation | acción del consumer | "yo no rotateé" | audit con `consumer_id` · trace_id en SigNoz |
| **I**nformation disclosure | secret en log | A1 lee logs | redacción de campos sensibles · tracing skip |
| Information disclosure | trace de error | A1 ve detalle interno | `IntoResponse` solo expone códigos estables |
| **D**oS | endpoint público | A1 floods | rate limit 60 r/s p/c · Cloudflare/Traefik delante |
| DoS | Vault | thundering herd post-restart | cache 60 s + jitter en `secret()` |
| **E**levation | scope bypass | A2 con `notion:read` invoca `stripe:write` | `auth::scope::check` |
| Elevation | mTLS bypass | A2 sin cert | Traefik rechaza · gateway rechaza si flag activo |

## LINDDUN (privacidad)

| Categoría | Riesgo | Control |
|-----------|--------|---------|
| **L**inkability | trace_id linkable a consumer humano | tokens son por máquina, no por humano |
| **I**dentifiability | nombre completo en audit | el `consumer_id` es un slug técnico (ej. `ci-staging`) |
| **N**on-repudiation | abuso firmado | aceptable: queremos repudio negativo |
| **D**etectability | sniffing patrón rotación | TLS 1.3 + métricas agregadas |
| **D**isclosure | volcado audit a tercero | rol Postgres + RLS futura |
| **U**nawareness | usuario no sabe qué se loggea | documentado en `docs/api.md` y `SECURITY.md` |
| **N**on-compliance | RGPD §17 derecho a borrado | n/a · no datos personales |

## Mitigaciones por adversario

| Adversario | Mitigación primaria | Detección |
|-----------|----------------------|-----------|
| A1 | Traefik mTLS + Bearer + rate limit | logs Traefik + spans con CN inválido |
| A2 | scopes JSON granulares + cert short-TTL | audit `denied` + alerta SigNoz |
| A3 | Vault sellado · imagen distroless · non-root | filewatch en `/vault/data` |
| A4 | Shamir 3-de-5 · MFA Vault · audit de Vault | filelog de Vault audit en SigNoz |
| A5 | `cargo deny` · `cargo audit` · pin estricto · CodeQL · cosign sign | CI bloquea merge |
| A6 | CODEOWNERS · 2 reviews para auth/security · cosign verify en deploy | revisión humana |
| A7 | TOTP segundo factor para vía-fax · drill | nonce reuso en audit |
| A8 | Vault hot path es independiente de Proton | drift detector horario |

## Trade-offs explícitos

- **Aceptamos** que el operador único es SPoF humano hasta que ManitasFritas
  esté implementado (PR8).
- **Aceptamos** que Traefik es punto crítico TLS — un Traefik comprometido
  puede inyectar headers `X-Forwarded-Tls-Client-Cert` falsos. Mitigación:
  el binario solo escucha en red interna del compose (no public).
- **Aceptamos** latencia 30 s–2 min para vía-fax porque la criticidad de los
  comandos justifica el coste de un canal asíncrono.
- **Aceptamos** que un secret rotado tardará hasta 60 s en propagarse
  (cache TTL). Aceptable en el rate de rotaciones reales (<1/día).
