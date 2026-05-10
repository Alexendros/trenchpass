# ADR-0007 · Vía-fax como canal PGP-firmado de comandos críticos

- Fecha: 2026-05-02
- Estado: Aceptado
- Decisor: Alexendros

## Contexto

Si la consola del operador es comprometida, ¿cómo se ejecutan comandos
críticos como `revoke-cert`, `seal-vault`, `rotate-all` sin depender del
mismo canal HTTP que pueda estar siendo monitoreado/manipulado?

## Decisión

Implementar un canal **out-of-band** llamado *vía-fax*:

- El operador envía mail PGP-firmado a un buzón Proton dedicado del gateway.
- Un worker IMAP polea ese buzón cada 60 s.
- Cada mensaje se valida: firma PGP del fingerprint del operador, nonce no
  visto, timestamp ±5 min, y un segundo factor TOTP por canal aparte.
- Si todo cuadra, se ejecuta el handler correspondiente y se registra en audit
  con el hash de la firma.

## Consecuencias

- Recovery realista cuando la consola HTTP no es de fiar.
- Latencia 30 s–2 min (dependiente del SMTP de Proton).
- Costo de mantenimiento: una cuenta Proton extra + clave PGP del operador
  custodiada en YubiKey.
- Imposibilidad de spam/phishing: requiere clave privada PGP + TOTP físico.
- Riesgo: dependencia de Proton mail. Documentado como SPoF parcial mitigado
  por el hecho de que los handlers son comandos defensivos (revocación,
  rotación) — bloquean en lugar de exponer.

## Alternativas descartadas

- **SSH dedicado**: añade superficie servidor. PGP-by-mail es asíncrono y
  resiste latencia/cortes.
- **Webhook HMAC**: requiere mantener el endpoint vivo y registrado. PGP es
  más portable.
- **Hardware token directo**: ideal pero requiere infraestructura física in-situ.
