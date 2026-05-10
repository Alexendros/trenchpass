#!/usr/bin/env bash
# Setup minimal de Vault dev para el smoke `refresh_smoke`.
# Asume que `vault server -dev -dev-root-token-id=root` ya corre en :8200.
set -euo pipefail

export VAULT_ADDR=${VAULT_ADDR:-http://127.0.0.1:8200}
export VAULT_TOKEN=${VAULT_TOKEN:-root}

echo "[setup] habilitando pki en mount pki_int"
vault secrets enable -path=pki_int pki || true
vault secrets tune -max-lease-ttl=87600h pki_int

echo "[setup] generando root CA"
vault write -field=certificate pki_int/root/generate/internal \
    common_name="trenchpass test root" \
    issuer_name="root-2026" \
    ttl=87600h > /dev/null

echo "[setup] configurando role mcp-gateway (allow trenchpass.local + subdominios + localhost)"
vault write pki_int/roles/mcp-gateway \
    allowed_domains="trenchpass.local,localhost" \
    allow_bare_domains=true \
    allow_subdomains=true \
    allow_localhost=true \
    max_ttl=168h \
    ttl=168h \
    key_type=rsa \
    key_bits=2048

echo "[setup] OK · ahora: cargo run --example refresh_smoke"
