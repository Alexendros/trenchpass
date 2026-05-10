# Seguridad

> TrenchPass es por definición una superficie crítica: custodia los secretos
> de toda la flota Alexendros. Tomamos los reportes muy en serio.

## Reportar una vulnerabilidad

**No** abras una issue pública.

| Canal | Detalle |
|-------|---------|
| Email PGP | `spiderwebtraveler@proton.me` (clave PGP en `docs/pgp/operator.asc`) |
| GitHub Security Advisory | <https://github.com/Alexendros/trenchpass/security/advisories/new> |
| Vía-fax (si existe) | mail PGP firmado al buzón Proton del gateway que se menciona en el reporte |

Plazos de respuesta:

- Acuse de recibo: ≤ 72 h.
- Evaluación inicial: ≤ 7 días.
- Fix + disclosure coordinado: 90 días por defecto, extensible si la
  complejidad lo requiere.

Reconocimiento público en `CHANGELOG.md` y opción a CVE si la severidad
justifica numeración.

## Modelo de amenazas (resumen)

### Activos
- Secretos de proveedor (Stripe, Notion, GitHub, …) almacenados en Vault.
- Material PKI (CA root + intermediates en Vault PKI).
- Audit log en Postgres-audit.
- Bóveda Proton Pass espejo.
- Shards Shamir 3-de-5 distribuidos.

### Adversarios considerados
1. **Atacante remoto sin credenciales** — explota un consumidor, intenta abusar de tools.
2. **Atacante remoto con cred válida pero scope limitado** — intenta escalar a otros namespaces.
3. **Atacante con acceso al VPS** — root local en el host de Dokploy.
4. **Atacante con acceso a Vault sellado** — necesita 3 shards Shamir.
5. **Atacante interno con vista parcial** — colaborador con repo + un secreto.
6. **Atacante temporal** — exfiltra un token, se quema en próxima rotación.

### Controles
| Amenaza | Control |
|---------|---------|
| Bearer fuga | mTLS doble factor; el cert solo lo emite Vault PKI con CN del consumidor. |
| Replay | nonce + timestamp ±5 min, cache 5 min. |
| Tampering audit log | rol Postgres `audit_writer` solo `INSERT`; backup R2 cifrado. |
| Compromiso del binario | Distroless + non-root; firma de imagen con cosign (PR2). |
| Compromiso del operador | ManitasFritas activa el plan de recuperación con Shamir. |
| Compromiso de Vault | Shamir 3-de-5; espejo Proton Pass; runbooks de recovery. |
| Phishing «vía-fax» | PGP firma + nonce + 2º factor TOTP + timestamp ±5 min. |
| Lateral movement post-compromiso | scopes JSON granulares; cert por consumidor con TTL 24 h–7 d. |

Modelo extendido (STRIDE + LINDDUN) en `docs/threat-model.md` (PR2).

## Hardening checklist (operador)

- [ ] Vault sellado por defecto; unseal Shamir manual tras boot.
- [ ] `TRENCHPASS_DEV_BEARER` **vacío** en producción (el código lo bloquea, además).
- [ ] mTLS obligatorio (`TRENCHPASS_MTLS_REQUIRED=true`) tras PR3.
- [ ] Audit Postgres con backups cifrados a R2 cada hora.
- [ ] otel-collector con `tls.insecure_skip_verify=false`.
- [ ] Rotación trimestral de certs PKI (TTL ≤ 7 d para CI, ≤ 24 h para humanos).
- [ ] `cargo audit` + `cargo deny` en CI bloquean merge.
- [ ] Imagen firmada con `cosign` y verificada en Dokploy.
- [ ] Heartbeat ManitasFritas activo (mensual mínimo).

## Out-of-scope

- DoS volumétrico contra el endpoint público (responsabilidad de Cloudflare/Traefik).
- Bugs en proveedores upstream (reportarlos a Stripe/GitHub/etc.).
- Vulnerabilidades en consumidores que ya tengan el secreto en runtime.

## Política de divulgación

Coordinada · 90 días estándar. Acreditamos al investigador en el `CHANGELOG.md`
y en el security advisory de GitHub salvo que prefiera anonimato.
