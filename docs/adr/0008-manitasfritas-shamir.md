# ADR-0008 · ManitasFritas · dead-hand switch con Shamir 3-de-5

- Fecha: 2026-05-03
- Estado: Aceptado
- Decisor: Alexendros

## Contexto

¿Qué pasa si el operador desaparece (incapacitación, fallecimiento, secuestro)?
La flota Alexendros necesita continuidad de operación o, al menos, un plan
ordenado de transición a custodios designados.

## Decisión

Implementar un **dead-hand switch** llamado *ManitasFritas*:

1. **Heartbeat** cada 30 días: el operador firma vía-fax `heartbeat`. Reset contador.
2. **Alerta** a los 60 días sin heartbeat: aviso firmado por el gateway a los
   recipientes designados.
3. **Disparo** a los 90 días sin heartbeat:
   - El gateway genera manifiestos cifrados (uno por recipiente).
   - Cada manifiesto contiene instrucciones + un shard Shamir 3-de-5.
   - El gateway envía los manifiestos vía Proton firmados.
   - Los recipientes reúnen ≥3 shards para reconstruir la unseal key del
     Vault y la clave maestra de Proton Pass.
4. **Anti-falsos-positivos**: ventana de 7 días con avisos diarios + segundo
   factor TOTP desde dispositivo confiado del operador.
5. **Drill semestral**: simulacro a recipientes sin disparar real.

## Consecuencias

- Continuidad de servicio o transición ordenada incluso ante incapacitación
  total del operador.
- Coste organizacional: identificar y mantener relación con los 5 recipientes;
  drill semestral con todos.
- Riesgo de falso positivo (operador de viaje) mitigado por ventana 7 días +
  TOTP fallback.
- Riesgo de colusión 3-de-5: aceptado como menor que el riesgo SPoF del
  operador único. Se elige a recipientes con conflictos de interés mutuos
  donde sea posible.

## Configuración pendiente (no se commit-ea)

- Lista de recipientes (viven en sobres físicos sellados, no en repo).
- Direcciones Proton de cada uno.
- Texto de los manifiestos (plantilla en `docs/runbooks/manitasfritas/`).

## Alternativas descartadas

- **Custodia en notaría**: caro, lento, jurisdicción frágil.
- **Servicio comercial dead-hand** (Dead Man's Switch SaaS): vendor lock-in.
- **Shamir 2-de-3**: demasiada confianza en pocos custodios; 3-de-5 da más
  margen ante un recipiente perdido o no cooperativo.
- **Shamir 5-de-7**: demasiado coordinación; tres es ya el límite de
  reuniones telegráficas razonables.
