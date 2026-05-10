# ADR-0006 · SigNoz desde día uno · Sentry/Glitchtip a retiro

- Fecha: 2026-05-01
- Estado: Aceptado
- Decisor: Alexendros

## Contexto

Controlink usa Sentry/Glitchtip para errores. Para el gateway necesitamos
**traces, métricas y logs correlacionados** desde la primera llamada — un
fallo en `vault.read → stripe.api` debe verse como un único trace.

## Decisión

**SigNoz self-hosted** desde día uno. Pipeline:

```
trenchpass (tracing-opentelemetry)
   ↓ OTLP gRPC
otel-collector
   ↓
ClickHouse (traces · metrics · logs)
   ↓
SigNoz query-service + frontend
```

Sentry/Glitchtip se mantienen en paralelo durante 2 semanas tras el cutover
de Controlink (PR6) y se retiran en PR9 si SigNoz cubre todos los casos
(error tracking + alerting).

## Consecuencias

- Una sola tubería para errores, performance y logs estructurados.
- Trace correlation natural entre el gateway y futuros consumidores que
  emitan OTLP.
- Coste de mantenimiento: ClickHouse necesita ~2 GB RAM, ~10 GB disco/mes.
- Si SigNoz falla, perdemos visibilidad. Mitigación: alerts independientes
  en GlitchTip durante el periodo de paralelo.

## Alternativas descartadas

- **Solo Sentry**: sin métricas ni traces unificados.
- **Datadog**: cumple objetivos pero SaaS-locked y caro.
- **Jaeger + Prometheus + Loki**: tres backends que mantener; SigNoz consolida.
