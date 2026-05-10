# ADR-0004 · mTLS terminado en Traefik con acceptor rustls fallback

- Fecha: 2026-04-30
- Estado: Aceptado
- Decisor: Alexendros

## Contexto

El gateway necesita validar **cert mTLS** del consumidor (CN = consumer ID).
Hay dos topologías razonables:

1. **mTLS hasta el binario**: rustls acceptor en el propio gateway.
2. **mTLS terminado en Traefik**: Traefik valida y propaga el cert al gateway en un header.

Alexendros ya opera Traefik delante de todo, con su propia configuración mTLS
en `infra/traefik/dynamic/tls.yml`. Duplicar la lógica TLS en el gateway añade
superficie sin valor.

## Decisión

**mTLS terminado en Traefik por defecto**. El gateway lee el cert validado
desde el header `X-Forwarded-Tls-Client-Cert` (URI-encoded PEM).

Como **fallback**, el módulo `transport::mtls` ofrece un acceptor rustls que
puede activarse por env var en escenarios sin Traefik (dev local, otros tenants).

## Consecuencias

- Una única superficie TLS = menos errores de configuración.
- Renovación de certs de servidor concentrada en Traefik + Let's Encrypt.
- El gateway **confía** en Traefik. Si Traefik está comprometido, el modelo
  de seguridad cae. El operador debe asegurar que el binario solo escucha
  desde la red interna del compose (`network: traefik-public`).
- En dev local sin Traefik, los desarrolladores activan
  `TRENCHPASS_MTLS_REQUIRED=false` y pasan los headers manualmente con `curl`.

## Alternativas descartadas

- **mTLS solo en el binario**: mover la termination al gateway añade gestión
  de SAN/SNI por cada host name, más renovaciones, más mTLS handshakes
  CPU-pesados que el operador prefiere centralizar en Traefik.
- **Sin mTLS, solo Bearer**: rechazado por el operador. Doble factor obligatorio.
