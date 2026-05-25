# Runbook · Vía-fax (comando out-of-band PGP-firmado)

> Spec: [ADR-0007](../adr/0007-via-fax-pgp-channel.md). Implementación:
> `src/fax/`. Activado en PR7.

## Cuándo usar

Cuando la consola HTTP del gateway no es de fiar o no responde:

- Sospecha de compromiso del Bearer/cert del operador (`revoke`/`invalidate`).
- Vault corrupto · necesidad de invalidar cache local (`invalidate-all`).
- Sellado forzado (`seal-vault`) ante incidente P0.

## Pre-requisitos

1. **Buzón Proton dedicado** del gateway con IMAP habilitado
   (Proton Premium · `imap.protonmail.ch:993`).
2. **Clave PGP del operador** custodiada en YubiKey · subkey de firma
   activo. Fingerprint hex (40 chars sin espacios) configurado en
   `FAX_PGP_OPERATOR_FINGERPRINT`.
3. **Cert público exportado** al gateway:
   ```bash
   gpg --export --armor "$FPR" > operator-pubkey.asc
   sudo install -m 0640 operator-pubkey.asc /etc/trenchpass/operator-pubkey.asc
   sudo chown trenchpass:trenchpass /etc/trenchpass/operator-pubkey.asc
   ```
   Y `FAX_PGP_OPERATOR_CERT_PATH=/etc/trenchpass/operator-pubkey.asc`.
4. **Variables IMAP** rellenas (`FAX_IMAP_USER`, `FAX_IMAP_PASSWORD`).
   Sin estas, el worker arranca pero queda inactivo (`warn!` por boot).

## Procedimiento

### 1. Componer comando YAML

`command.yaml`:

```yaml
nonce: 6f5d3e26-cb04-4b78-bbac-3a3c8b4f0001
timestamp: 1748160000
command: invalidate
path: kv/notion/api_key
```

- `nonce`: UUIDv4 único · `uuidgen` o `python -c 'import uuid;print(uuid.uuid4())'`.
- `timestamp`: epoch UTC actual · `date +%s`.
- `command` + campos: ver [Verbos](#verbos).

### 2. Firmar con GPG

```bash
gpg --sign --armor --output command.asc command.yaml
```

Output esperado: `command.asc` empieza por `-----BEGIN PGP MESSAGE-----`.

### 3. Enviar al buzón Proton

Adjuntar `command.asc` (o pegar inline en cuerpo `text/plain`) al mail
dirigido al buzón del gateway. Asunto irrelevante (el worker no lo lee).

### 4. Verificar ejecución

- Latencia esperada: `FAX_POLL_INTERVAL_SECS` (60 s por defecto).
- Logs SigNoz · filtro `target = "fax.dispatch"`:
  - `vault cache invalidated` o `vault cache entry invalidated` (OK).
  - `revoke aún no cableado · ver PR7.1` (TODO).
- Audit log Postgres:
  ```sql
  SELECT * FROM audit_events
  WHERE consumer_id = 'via-fax'
  ORDER BY id DESC LIMIT 5;
  ```
- Si el mensaje queda **UNSEEN** tras dos ciclos: firma inválida o YAML
  malformado · revisar logs `fax.imap warn` con `error = ...`.

## Verbos

| `command:` | Campos | Efecto | Estado |
|-----------|--------|--------|--------|
| `invalidate-all` | — | Limpia el cache `DashMap` de `VaultClient`. NO rota secretos en Vault. | ✅ |
| `invalidate` | `path: <kv-path>` | Quita una entrada del cache. | ✅ |
| `revoke` | `serial: <serial-hex>` | Revoca un cert PKI en Vault. | ⏳ PR7.1 |
| `seal-vault` | — | `vault operator seal` (destructivo · requiere unseal con Shamir). | ⏳ PR7.1 |

## Anti-replay

`src/security/replay.rs` impone:

- Timestamp dentro de **±5 min** del wall-clock del gateway.
- Nonce **único** dentro de los últimos 5 min.

Si reenvías el mismo mail dos veces, el segundo será rechazado con
`FaxError::Replay`. Genera un nonce nuevo y reescribe el timestamp.

## Troubleshooting

| Síntoma | Causa probable | Fix |
|---------|---------------|-----|
| Worker no arranca (no log `vía-fax worker arrancando`) | Config incompleta | Verifica `FAX_IMAP_HOST`, `FAX_IMAP_USER`, `FAX_PGP_OPERATOR_FINGERPRINT` no vacíos. |
| `TLS handshake: ...` | Cert IMAP no validable | Inyectar CA bundle del sistema · ver `src/fax/imap.rs::rustls_native_or_static_roots` (PR7.1 añade `rustls-native-certs`). |
| `login: ...` | Credencial IMAP errónea | Regenerar Proton IMAP password desde Proton Account · cuidado con MFA. |
| `no se encontró armadura PGP en el mail` | Olvido de firmar / formato erróneo | Asegúrate que el mensaje empieza por `-----BEGIN PGP MESSAGE-----`. |
| `operator_cert fpr (X) ≠ expected (Y)` | El cert cargado en disco no es el del operador esperado | Re-exportar con `gpg --export --armor <fpr>` y reinstalar. |
| `FaxError::NoOperatorCert` | `FAX_PGP_OPERATOR_CERT_PATH` no apunta a un cert válido | Verifica el path; el cert debe ser armored OpenPGP, no PEM x509. |
| `FaxError::Replay` | Reenvío o timestamp drifteado | Genera nonce nuevo y timestamp actual. |

## Auditoría forense

Cada despacho registra en `audit_events`:

- `consumer_id = 'via-fax'`
- `action = 'fax.<verb>'`
- `outcome = ok | error`
- `detail.command` (JSON del comando)
- `detail.signature_sha256` (SHA-256 hex de los bytes armored firmados ·
  permite cotejar contra el mail conservado en el buzón Proton)

## Drill

Cada trimestre el operador debe:

1. Enviar un `invalidate-all` de prueba al buzón.
2. Confirmar `outcome=ok` en audit log dentro de 2 min.
3. Documentar el ejercicio en `docs/runbooks/recovery-drill.md`.
