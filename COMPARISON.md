# Comparativa · TrenchPass frente a productos similares

> Última revisión: 2026-05-10. Este documento se regenera cada release.
> Si una columna queda obsoleta, ábrenos issue.

TrenchPass cae en la intersección de tres categorías que rara vez se solapan:

1. **Secret managers** (Vault, OpenBao, Infisical, Doppler, Bitwarden Secrets…)
2. **MCP gateways / proxies** (mcp-proxy, MCP-Bridge, supergateway, Smithery…)
3. **API gateways con rewriting de credenciales** (Pomerium, Kong + plugins, mcp-gateway de Lasso)

Ningún producto existente cubre los tres simultáneamente con la postura de
TrenchPass. Esta tabla intenta justificar por qué construimos algo nuevo en
lugar de adoptar uno existente.

---

## Resumen ejecutivo

| Criterio | TrenchPass | Vault Agent | OpenBao | Infisical | Doppler | mcp-proxy | MCP-Bridge | supergateway | Smithery | Pomerium | Bitwarden Secrets |
|---|:-:|:-:|:-:|:-:|:-:|:-:|:-:|:-:|:-:|:-:|:-:|
| **MCP-native** | ✅ | ❌ | ❌ | ❌ | ❌ | ✅ | ✅ | ✅ | ✅ | ❌ | ❌ |
| **Secret store autónomo** | espejo | ✅ | ✅ | ✅ | ✅ | ❌ | ❌ | ❌ | ❌ | parcial | ✅ |
| **mTLS doble factor por consumidor** | ✅ (Vault PKI) | parcial | parcial | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ✅ | ❌ |
| **Audit append-only Postgres** | ✅ | file/syslog | file/syslog | DB propia | DB propia | ❌ | ❌ | ❌ | ❌ | ✅ | DB propia |
| **Observabilidad OTLP nativa** | ✅ | parcial | parcial | parcial | parcial | ❌ | ❌ | ❌ | ❌ | ✅ | ❌ |
| **Rotación sin redeploy del consumidor** | ✅ (cache 60 s) | ✅ template | ✅ template | ✅ webhook | ✅ webhook | n/a | n/a | n/a | n/a | ✅ | ✅ webhook |
| **Recovery k-de-n (Shamir)** | ✅ 3-de-5 | ✅ Vault | ✅ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| **Comando out-of-band PGP** | ✅ vía-fax | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| **Dead-hand switch** | ✅ ManitasFritas | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ | ❌ |
| **Self-hosted FOSS** | ✅ AGPL | ✅ MPL | ✅ MPL | ✅ MIT | ❌ SaaS | ✅ MIT | ✅ MIT | ✅ MIT | ❌ SaaS | ✅ Apache | ✅ AGPL |
| **Imagen distroless ≤ 20 MB** | ✅ ~12 MB | ❌ ~140 MB | ❌ ~120 MB | ❌ ~250 MB | n/a | ~50 MB | ~80 MB | ~40 MB | n/a | ~80 MB | ~150 MB |
| **Lenguaje** | Rust | Go | Go | TS+Go | TS | Python | TS | Python | TS | Go | C# |
| **Multi-namespace por consumer (scopes JSON)** | ✅ | ✅ policies HCL | ✅ | ✅ | ✅ | ❌ | ❌ | ❌ | ❌ | ✅ | ✅ |

> Leyenda: ✅ soportado de fábrica · parcial = posible con configuración no
> trivial o plugin · ❌ no aplica o requiere reescritura.

---

## 1. Secret managers tradicionales

### 1.1 HashiCorp Vault Agent
- **Pros**: la referencia; PKI maduro; auto-auth + templating excelente.
- **Contras**: cada consumidor necesita un sidecar Vault Agent o un fetcher con `vault read`; **no habla MCP**, no tiene noción de "tool"; observabilidad OTLP requiere `prometheus telemetry` + adaptador externo; HCP cloud es de pago.
- **Cuándo elegirlo**: organización grande con stack ya en HashiCorp; prefieren mantener `vault read secret/foo` en cada app; no quieren tocar el contrato MCP.
- **Por qué no nos sirve**: queremos **una llamada `notion.search_pages`**, no `vault read secret/notion → fetch /search`. Mover el rewriting al sidecar es más superficie por consumidor.

### 1.2 OpenBao
- Idéntico a Vault Agent en arquitectura. Fork comunitario tras el cambio de licencia BSL de Vault.
- **TrenchPass usa OpenBao internamente como motor del store** (compatible con vaultrs sobre la API v1). Sustituirá a Vault OSS si el operador pierde confianza en HashiCorp.

### 1.3 Infisical
- **Pros**: UX moderna, K8s operator, webhooks de rotación; abierto MIT.
- **Contras**: pensado para inyectar `process.env`; sin PKI propio robusto; sin vista de "tools"; trace propio no es OTLP.
- **Por qué no**: pensado para que cada app reciba `INFISICAL_TOKEN` y resuelva `infisical run -- node app.js`. El acoplamiento del nombre del proveedor a `process.env.<KEY>` se mantiene.

### 1.4 Doppler
- **Pros**: SaaS pulido, secret references, dynamic secrets via integraciones.
- **Contras**: SaaS-first; precio creciente con número de proyectos; sin opción de auto-hosting libre.
- **Por qué no**: Alexendros prioriza autocustodia (CLAUDE.md §Stack online).

### 1.5 Bitwarden Secrets Manager
- **Pros**: AGPL como nosotros; CLI decente; familia confianza-cero.
- **Contras**: orientado a humanos; API SDK joven; rotación poco automatizable; sin PKI.

### 1.6 AWS Secrets Manager / GCP Secret Manager
- Cloud-locked. Out of scope: Alexendros opera en VPS Hostinger + Cloudflare R2.

---

## 2. MCP gateways y proxies

Esta categoría es **emergente** (todo de 2024–2026). La mayoría son proxies
de transporte (stdio ⇄ HTTP/SSE) sin ninguna noción de custodia de secretos.

### 2.1 `mcp-proxy` (sparfenyuk)
- **Pros**: el proxy stdio↔SSE más usado en clients Claude Desktop. Python.
- **Contras**: no custodia secretos, no autentica clientes, no audita.
- **Caso de uso**: exponer un MCP local a un IDE remoto, no un servicio de producción.

### 2.2 `MCP-Bridge`
- **Pros**: federa varios MCP servers tras una API OpenAI-compatible.
- **Contras**: no resuelve el problema de credenciales — cada server backend sigue necesitando sus secretos cableados.

### 2.3 `supergateway`
- **Pros**: traduce stdio MCP a SSE/WS.
- **Contras**: cero custodia, cero auth.

### 2.4 Smithery (smithery.ai)
- **Pros**: catálogo SaaS de MCP servers con OAuth.
- **Contras**: SaaS, vendor lock-in, cada server sigue manejando sus tokens.
- **Por qué no**: Alexendros no externaliza credenciales productivas.

### 2.5 `mcp-gateway` (Lasso Security)
- **Pros**: el más cercano en filosofía. Añade auth, rate limit, observabilidad sobre un mesh MCP.
- **Contras**: comercial; no integra Vault PKI; no habla scopes JSON granulares con cert match; no tiene recovery Shamir ni dead-hand.
- **Convergencia futura**: si liberan licencia FOSS y soportan PKI, repensaremos PR3+.

### 2.6 `dive` / `mcp-router` y similares (open WebUI ecosystem)
- Routers para escritorios LLM. No son productos de servidor productivo.

---

## 3. API gateways con rewriting de credenciales

### 3.1 Pomerium
- **Pros**: zero-trust serio, mTLS sólido, OPA-policy, audit JSON.
- **Contras**: orientado HTTP/JSON-RPC genérico; no entiende MCP framing; el rewriting de credenciales a backends se hace por header injection, no por translation a una API tipada.
- **Convergencia**: podríamos poner TrenchPass **detrás** de Pomerium si Alexendros añade SSO humano. Hoy Traefik basta.

### 3.2 Kong + plugins (`request-transformer-advanced`)
- **Pros**: maduro, ecosistema enorme.
- **Contras**: licensing complicado para features avanzadas; configuración por DB; footprint pesado.
- **Por qué no**: overkill para 1 servicio.

### 3.3 Apigee, AWS API Gateway, Azure API Management
- SaaS, vendor lock-in. Out of scope.

---

## 4. Patrones service-mesh con sidecar de Vault

| Producto | Comentario |
|----------|-----------|
| Istio + Vault sidecar | Inyecta certs vía SDS; no resuelve scopes ni MCP. Pesado para 1 host. |
| Linkerd2 + secret CRDs | Igual: layer transport, no application. |
| consul-template | Renderiza configs con secretos en disk. Mejor que `process.env` pero acopla a fichero. |

Patrón estándar: **el consumidor sigue conociendo el nombre del proveedor**. TrenchPass rompe eso por diseño.

---

## 5. Cuadrante de posicionamiento

```
                      Custodia y rotación
                              ▲
                              │
                              │   ◆ Vault Agent
                              │   ◆ OpenBao
                              │
                              │             ◆ Infisical
                              │   ◆ Bitwarden SM      ◆ Doppler
                              │
       ★ TrenchPass ────────────┼─────────────────────────────────►
                              │                  Conocimiento de MCP
                              │             ◆ mcp-gateway (Lasso)
                              │   ◆ supergateway   ◆ Smithery
                              │   ◆ MCP-Bridge
                              │   ◆ mcp-proxy
                              │
                              │   ◆ Pomerium
                              │   ◆ Kong
                              │
                              ▼
                        Auth de transporte
```

TrenchPass es el único punto que vive en el cuadrante superior derecho:
**custodia + rotación profesional + nativo MCP + mTLS doble factor**.

---

## 6. Decisión "build vs buy"

| Opción | Coste estimado | Desventaja insalvable |
|--------|----------------|------------------------|
| Vault Agent + 16 fetchers TS reescritos para `vault read` | 2 sprints | Cada nuevo proveedor toca 16 lugares; cero noción MCP; sin scopes-tipo-tool. |
| Infisical + reescribir consumers a Infisical SDK | 2 sprints + SaaS bill | Vendor lock-in al SDK; sin recovery Shamir profesional; rotación via webhooks frágil. |
| `mcp-gateway` (Lasso) comercial | $$ + integración custom | No es FOSS; no integra Vault PKI nativo; no podemos auditar. |
| **Construir TrenchPass** | 9 PRs (~3 sprints) | Mantenimiento eterno por nuestra cuenta. |

Elegimos construir porque **Alexendros prioriza autocustodia, licencias
permisivas y código abierto** (CLAUDE.md §Stack online) y porque ningún
producto cubre los tres ejes simultáneamente.

---

## 7. Lo que aprendemos de cada uno (deuda inversa)

| Producto | Idea robada |
|----------|-------------|
| Vault Agent | template + `auto_auth` (PR2 lo aplicamos al cache TTL). |
| OpenBao | el solo hecho de existir nos da el plan B si HashiCorp endurece BSL. |
| Infisical | webhook on-rotation → SigNoz alert (PR4). |
| Pomerium | scoping por path + JWT claim → JSON scopes con CN match. |
| mcp-proxy | log de todas las request/response como artefacto debug (PR9 toggle). |
| Bitwarden | UI minimalista + Argon2 KDF para futuro hardening offline. |
| Doppler | secret references como first-class objects (lo emulamos con `secret_path` en audit). |

---

## 8. Cuándo **NO** uses TrenchPass

- Si solo tienes 1 app y 3 secretos: usa `.env` + Doppler/Infisical free tier.
- Si tu equipo no opera Vault: el coste operacional inicial te abruma.
- Si tu carga es <5 r/s sostenidos: cualquier sidecar de Vault sirve.
- Si necesitas SSO humano + audit GDPR completo: pon Pomerium delante.
- Si tu stack es 100 % AWS: usa Secrets Manager + Lambda Authorizers.

TrenchPass brilla cuando hay **flota de N apps + ~M proveedores externos** y la
operación quiere **un solo lugar para rotar** sin tocar el código de las apps.
