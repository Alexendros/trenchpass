# Runbook · ManitasFritas (operador)

> Documento del **operador**. La copia de los recipientes vive aparte y
> contiene plantillas de comunicación firmadas. **Sensible**: no commit-ear
> el archivo `recipients.md` con datos reales.

## Resumen

ManitasFritas es el dead-hand switch del proyecto. Si el operador falta:

- **30 d** sin heartbeat → primera notificación SigNoz (advertencia interna).
- **60 d** → alerta a recipientes (todavía sin disparar).
- **90 d** → disparo: el gateway envía manifiestos cifrados a los 5 recipientes
  con instrucciones, contactos cruzados y un shard Shamir cada uno.

## Requisitos previos

| Item | Estado |
|------|--------|
| 5 recipientes designados | pendiente · operador interno |
| 5 shards Shamir distribuidos | pendiente · PR8 |
| Plantilla manifest cifrada por recipiente | pendiente · PR8 |
| TOTP fallback en dispositivo confiado | pendiente · PR8 |
| Drill semestral en calendario | sí · 1 enero / 1 julio |

## Heartbeat (operador, mensual)

Procedimiento mínimo:

```bash
# 1. Componer mail PGP firmado al buzón vía-fax del gateway.
echo "heartbeat $(date -Iseconds) nonce=$(uuidgen)" | \
  gpg --clear-sign --local-user <fingerprint> | \
  proton-mail-cli send \
    --to fax-trenchpass@proton.me \
    --subject "[heartbeat]"

# 2. Confirmar en SigNoz dashboard "ManitasFritas · Heartbeat" que el contador
#    se reseteó.
```

Si por alguna razón no puedes enviar PGP, hay un **fallback TOTP** desde
dispositivo confiado: web del gateway `/admin/heartbeat` requiere TOTP del
operador + recovery codes.

## Si el contador llega a 60 d

Recibirás email (capacidad activa) con asunto `[ManitasFritas] T-30 hasta disparo`.

- Si **estás vivo y operativo**: emite heartbeat de inmediato.
- Si **estás de viaje sin acceso**: notifica a un recipiente que extienda la
  ventana usando su shard + endpoint `/admin/extend-deadhand` (PR8).

## Si el contador llega a 90 d

El gateway dispara automáticamente. No requiere intervención del operador
(que probablemente no está). Los recipientes reciben:

1. Manifest PGP cifrado para ellos individualmente.
2. Plantilla de comunicación entre recipientes.
3. Direcciones cruzadas (cada recipiente conoce a los otros 4).
4. Su shard Shamir.

## Plantilla manifest (rellenar en PR8, no commit)

Vive en `docs/runbooks/manitasfritas/manifest-template.md.gpg` cifrado para
el operador. Contiene:

- Identidad de los 5 recipientes.
- Instrucciones de reunión y reconstrucción Shamir.
- Mapa de la infraestructura productiva (qué runs dónde).
- Política de continuidad: opciones (a) cerrar Alexendros, (b) mantener
  servicio mínimo bajo nuevo lead, (c) liquidación ordenada.

## Drill

Una vez al año (no semestral porque es invasivo) los recipientes participan
en un drill simulado:

1. Operador emite mensaje firmado `drill-test` en lugar del disparo real.
2. Recipientes confirman recepción.
3. Sin reconstrucción real de Shamir.
4. Cualquier recipiente que no responda en 14 días → reemplazo.
