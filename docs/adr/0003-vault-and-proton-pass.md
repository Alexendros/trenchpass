# ADR-0003 · Vault OSS hot path + Proton Pass espejo (papel autocopiante)

- Fecha: 2026-04-29
- Estado: Aceptado
- Decisor: Alexendros

## Contexto

Necesitamos un store de secretos con tres propiedades:

1. **Versionado** y rotación atómica.
2. **PKI** capaz de emitir certs cliente con CN-based mTLS.
3. **Recuperación** robusta con k-de-n incluso si el operador pierde acceso.

Ningún producto único cubre las tres con la postura ergonómica que el operador
quiere (biométrico para humano + máquina para gateway).

## Decisión

Modelo **híbrido**:

- **Vault OSS** como custodio primario (hot path). KV v2 + PKI mount.
- **Proton Pass** como espejo bidireccional (cuenta Proton del operador).
- Drift detection horaria entre ambos, alertada en SigNoz.
- Shamir 3-de-5 para la unseal key, distribuido entre 5 medios independientes.

## Consecuencias

- Dos sistemas de secret management que mantener.
- Sync worker vive en el binario del gateway (no en un servicio aparte) para
  reducir latencia post-rotación.
- Shamir requiere disciplina del operador en distribución de shards. Drill
  semestral documentado en `docs/runbooks/recovery-drill.md`.
- El operador siempre puede recuperar con biometric (Proton) **o** con shards
  físicos (Shamir). Doble red de seguridad.
- La cuenta Proton bloqueada **no** afecta el funcionamiento normal: el hot
  path sigue siendo Vault.

## Alternativas descartadas

- **Solo Vault**: SPoF si Vault corrupto y backups inaccesibles.
- **Solo Proton Pass**: API privada, sin PKI mount, sin policies HCL.
- **AWS Secrets Manager + KMS**: vendor lock-in, no encaja con autocustodia.
- **OpenBao en lugar de Vault**: válido, lo guardamos como plan B activable
  cambiando el endpoint en `vaultrs::client`.
