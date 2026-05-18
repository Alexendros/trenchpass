# Glosario · TrenchPass

Vocabulario controlado del proyecto. Si una palabra aparece sin estar aquí, abre PR.

## A

**AppRole** — método de autenticación de Vault que TrenchPass usa para
identificarse en lugar de un token root. Configurado en `infra/vault/policies/`.

**Audit append-only** — propiedad del log que el rol Postgres `audit_writer`
asegura: solo `INSERT`, sin `UPDATE`/`DELETE`.

## B

**Bearer scope** — string del tipo `notion:*` o `stripe:list_subscriptions`
que el gateway compara con la tool invocada antes de autorizar.

## C

**Cache TTL** — tiempo de vida del cache de secretos Vault dentro del gateway.
Default 60 s, configurable. Una rotación tarda como máximo este tiempo en
propagarse a los consumidores sin reinicio.

**Consumer** — máquina (no humano) que invoca al gateway. Cada consumer tiene
un `consumer_id` único, un cert mTLS con CN = consumer_id, y un Bearer token
con scopes. Ejemplos: `controlink-prod`, `ci-staging`, `n8n-flow-42`.

## D

**Dead-hand switch** — mecanismo que se activa por inacción del operador.
Aquí lo llamamos *ManitasFritas* (ver M).

**Drift detection** — cron horario que compara el hash de la bóveda Proton Pass
con el snapshot Vault. Discrepancia → alerta SigNoz.

**Distroless** — imagen base mínima de Google (sin shell, sin coreutils) que
usamos como runtime. Tamaño final ~12 MB.

## H

**Heartbeat** — mensaje firmado vía-fax que el operador emite cada 30 días
para resetear el contador de ManitasFritas.

## M

**ManitasFritas** — el dead-hand switch del proyecto. 30 d heartbeat → 60 d
alerta → 90 d disparo. Plantilla operator-runbook en `docs/runbooks/manitasfritas/`.

**MCP** — Model Context Protocol (Anthropic, 2024). Define cómo los tools se
exponen a los modelos LLM. El gateway se expresa en este protocolo.

**mTLS** — mutual TLS. En TrenchPass, cada consumer presenta un cert emitido
por Vault PKI con su `consumer_id` como CN. El gateway compara este CN con
el sujeto del Bearer.

## N

**Namespace** — agrupación de tools por proveedor (`notion`, `stripe`, …).
13 namespaces declarados en v1.

**Nonce** — UUID v4 que el consumidor envía en cada request. El gateway
mantiene cache 5 min para detectar replay.

## O

**OpenBao** — fork comunitario de HashiCorp Vault tras el cambio de licencia
BSL. Plan B activable cambiando solo el endpoint en `vaultrs::client`.

**Operador** — Alexendros. Persona física dueña del repo, las claves Shamir
y el dispositivo TOTP de fallback.

## P

**Papel autocopiante** — metáfora del modelo Vault + Proton Pass. Cada
escritura va a ambos sitios; cualquiera puede recuperar al otro.

**PKI mount** — mount de Vault que emite certs cliente. Lo configuramos para
emitir certs de TTL ≤ 7 d para CI y ≤ 24 h para humanos.

**Postgres-audit** — instancia Postgres dedicada al audit log. Aislada del
Postgres operativo de Controlink.

**Proton Pass** — espejo offline de los secretos en la cuenta Proton del
operador. Recovery con biometric. Sin él, la unseal Vault Shamir sigue
siendo posible vía shards físicos.

## R

**rmcp** — Rust SDK oficial de Anthropic para MCP. Versión actual 1.7.x.

**Rotación atómica** — `vault kv put` reescribe la KV v2 con una nueva versión.
El gateway detecta la versión nueva y purga su cache.

## S

**Scope** — ver Bearer scope.

**Shamir 3-de-5** — Shamir Secret Sharing con k=3, n=5. La unseal key de
Vault se parte en 5 shards distribuidos; basta reunir 3 para reconstruirla.

**SigNoz** — backend de observabilidad self-hosted (ClickHouse + frontend).
Recibe traces, métricas y logs vía OTLP.

**Smithery** — catálogo SaaS de MCP servers. Comparado en `COMPARISON.md` §2.4.

## T

**Tool** — operación atómica que el gateway expone. Cada tool tiene un `id`
(`<namespace>.<verb>_<resource>`), un schema de params, un schema de result,
y un handler que resuelve el secret y llama al proveedor.

**Traefik** — reverse proxy delante de todo en VPS. Termina TLS y mTLS y pasa
el cert validado al gateway.

## V

**Vault OSS** — HashiCorp Vault con licencia BSL OSS (la original, antes del
cambio que motivó OpenBao). Opera el hot path.

**Vía-fax** — canal out-of-band PGP-firmado que el operador usa para comandos
críticos (revoke, rotate-all, seal). Buzón Proton dedicado, polled cada 60 s.

## W

**Worker (sync)** — task tokio que escribe en Proton Pass cada vez que se
modifica un secreto en Vault. Lag <30 s.
