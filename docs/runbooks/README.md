# Runbooks

Procedimientos operativos paso-a-paso. Cada runbook es **autosuficiente**:
un operador novato debe poder ejecutarlo sin contexto adicional.

| Runbook | Cuándo se usa |
|---------|---------------|
| [vault-unseal.md](vault-unseal.md) | Vault sellado tras reinicio. |
| [rotate-provider-token.md](rotate-provider-token.md) | Rotación rutinaria de credencial. |
| [revoke-consumer.md](revoke-consumer.md) | Sospecha de Bearer/cert filtrado. |
| [recovery-drill.md](recovery-drill.md) | Drill semestral de recovery. |
| [manitasfritas/operator.md](manitasfritas/operator.md) | Procedimiento ManitasFritas (PR8). |
| [incident-response.md](incident-response.md) | Procedimiento P0/P1. |

> Los runbooks que tocan claves físicas (shards Shamir, custodios) tienen
> contenido sensible que **no se commit-ea** al repo público; viven en
> sobres físicos sellados y la copia digital cifrada en Proton Pass del operador.
