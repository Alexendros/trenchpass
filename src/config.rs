//! Configuración runtime cargada desde entorno (`.env` en dev, secrets en prod).

use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;

use serde::Deserialize;

use crate::error::{Error, Result};

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub tls: TlsConfig,
    pub vault: VaultConfig,
    pub database: DatabaseConfig,
    pub otel: OtelConfig,
    pub proton_pass: ProtonPassConfig,
    pub fax: FaxConfig,
    pub mfritas: MfritasConfig,
    pub env: Environment,
    pub dev_bearer: Option<String>,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Environment {
    Development,
    Staging,
    Production,
}

impl Environment {
    pub fn is_production(self) -> bool {
        matches!(self, Self::Production)
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ServerConfig {
    pub bind: SocketAddr,
    pub log_level: String,
}

#[derive(Debug, Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TlsMode {
    /// Sin TLS · sólo dev tras proxy reverso terminador (Traefik, Caddy).
    Off,
    /// PEMs locales fijos · break-glass / dev / smoke.
    Static,
    /// Vault PKI emite leaf + ca_chain con TTL ≤ 7 d, refresh automático.
    VaultPki,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TlsConfig {
    pub mode: TlsMode,
    /// PEMs estáticos (sólo `mode=static`).
    pub cert: Option<PathBuf>,
    pub key: Option<PathBuf>,
    /// Trust anchors para verificar clientes mTLS (sólo `mode=static`).
    /// En `mode=vault_pki` se usa el `ca_chain` devuelto por Vault.
    pub client_ca: Option<PathBuf>,
    /// Si `true`, exige cert cliente válido. Si `false`, mTLS opcional.
    pub mtls_required: bool,
    /// Vault PKI · rol que firma el leaf cert del gateway.
    pub pki_role: String,
    /// CN solicitado al emitir (`alt_names` ext SAN se derivan).
    pub pki_common_name: String,
    /// SAN DNS adicionales (CSV).
    pub pki_alt_names: Vec<String>,
    /// TTL solicitado al emitir. Vault aplica `min(ttl, role.max_ttl)`.
    pub pki_cert_ttl: Duration,
    /// Refrescar al alcanzar este % del TTL emitido (default 50).
    pub pki_refresh_percent: u8,
    /// Jitter ± para evitar thundering herd (default 10 %).
    pub pki_refresh_jitter_percent: u8,
}

#[derive(Debug, Clone, Deserialize)]
pub struct VaultConfig {
    pub addr: String,
    pub token: String,
    pub kv_mount: String,
    pub pki_mount: String,
    pub cache_ttl: Duration,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseConfig {
    pub url: String,
    pub max_connections: u32,
}

#[derive(Debug, Clone, Deserialize)]
pub struct OtelConfig {
    pub endpoint: String,
    pub service_name: String,
    pub resource_attributes: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ProtonPassConfig {
    pub cli_bin: PathBuf,
    pub vault_id: String,
    pub drift_interval: Duration,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FaxConfig {
    pub imap_host: String,
    pub imap_port: u16,
    pub imap_user: String,
    pub imap_password: String,
    pub pgp_operator_fingerprint: String,
    pub poll_interval: Duration,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MfritasConfig {
    pub heartbeat_interval_days: u32,
    pub alert_days: u32,
    pub disparo_days: u32,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        // dotenvy es no-op si no existe `.env`
        let _ = dotenvy::dotenv();

        let env = parse_env::<String>("TRENCHPASS_ENV").unwrap_or_else(|_| "development".into());
        let env: Environment = match env.as_str() {
            "production" => Environment::Production,
            "staging" => Environment::Staging,
            _ => Environment::Development,
        };

        let server = ServerConfig {
            bind: parse_env::<String>("TRENCHPASS_BIND")
                .unwrap_or_else(|_| "0.0.0.0:8300".into())
                .parse()
                .map_err(|e| Error::Config(format!("TRENCHPASS_BIND: {e}")))?,
            log_level: parse_env::<String>("TRENCHPASS_LOG_LEVEL")
                .unwrap_or_else(|_| "info".into()),
        };

        let tls_mode = match parse_env::<String>("TRENCHPASS_TLS_MODE")
            .unwrap_or_else(|_| "off".into())
            .as_str()
        {
            "vault_pki" | "vault" | "pki" => TlsMode::VaultPki,
            "static" | "pem" => TlsMode::Static,
            _ => TlsMode::Off,
        };
        let tls = TlsConfig {
            mode: tls_mode,
            cert: parse_env::<PathBuf>("TRENCHPASS_TLS_CERT").ok(),
            key: parse_env::<PathBuf>("TRENCHPASS_TLS_KEY").ok(),
            client_ca: parse_env::<PathBuf>("TRENCHPASS_TLS_CLIENT_CA").ok(),
            mtls_required: parse_bool("TRENCHPASS_MTLS_REQUIRED").unwrap_or(false),
            pki_role: parse_env::<String>("TRENCHPASS_PKI_ROLE")
                .unwrap_or_else(|_| "mcp-gateway".into()),
            pki_common_name: parse_env::<String>("TRENCHPASS_PKI_COMMON_NAME")
                .unwrap_or_else(|_| "trenchpass.local".into()),
            pki_alt_names: parse_env::<String>("TRENCHPASS_PKI_ALT_NAMES")
                .unwrap_or_default()
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect(),
            pki_cert_ttl: Duration::from_secs(
                parse_env::<u64>("TRENCHPASS_PKI_CERT_TTL_SECS").unwrap_or(7 * 24 * 60 * 60), // 7 d
            ),
            pki_refresh_percent: parse_env::<u8>("TRENCHPASS_PKI_REFRESH_PERCENT").unwrap_or(50),
            pki_refresh_jitter_percent: parse_env::<u8>("TRENCHPASS_PKI_REFRESH_JITTER_PERCENT")
                .unwrap_or(10),
        };
        if tls.mode == TlsMode::VaultPki && tls.pki_refresh_percent < 10 {
            return Err(Error::Config(
                "TRENCHPASS_PKI_REFRESH_PERCENT debe ser ≥ 10".into(),
            ));
        }

        let vault = VaultConfig {
            addr: required("VAULT_ADDR")?,
            token: required("VAULT_TOKEN")?,
            kv_mount: parse_env::<String>("VAULT_KV_MOUNT").unwrap_or_else(|_| "secret".into()),
            pki_mount: parse_env::<String>("VAULT_PKI_MOUNT").unwrap_or_else(|_| "pki".into()),
            cache_ttl: Duration::from_secs(parse_env::<u64>("VAULT_CACHE_TTL_SECS").unwrap_or(60)),
        };

        let database = DatabaseConfig {
            url: required("DATABASE_URL")?,
            max_connections: parse_env::<u32>("DATABASE_MAX_CONNECTIONS").unwrap_or(8),
        };

        let otel = OtelConfig {
            endpoint: parse_env::<String>("OTEL_EXPORTER_OTLP_ENDPOINT")
                .unwrap_or_else(|_| "http://otel-collector:4317".into()),
            service_name: parse_env::<String>("OTEL_SERVICE_NAME")
                .unwrap_or_else(|_| "trenchpass".into()),
            resource_attributes: parse_env::<String>("OTEL_RESOURCE_ATTRIBUTES")
                .unwrap_or_default(),
        };

        let proton_pass = ProtonPassConfig {
            cli_bin: parse_env::<PathBuf>("PROTON_PASS_CLI_BIN")
                .unwrap_or_else(|_| PathBuf::from("/usr/local/bin/protonpass-cli")),
            vault_id: parse_env::<String>("PROTON_PASS_VAULT_ID").unwrap_or_default(),
            drift_interval: Duration::from_secs(
                parse_env::<u64>("PROTON_PASS_DRIFT_INTERVAL_SECS").unwrap_or(3600),
            ),
        };

        let fax = FaxConfig {
            imap_host: parse_env::<String>("FAX_IMAP_HOST").unwrap_or_default(),
            imap_port: parse_env::<u16>("FAX_IMAP_PORT").unwrap_or(993),
            imap_user: parse_env::<String>("FAX_IMAP_USER").unwrap_or_default(),
            imap_password: parse_env::<String>("FAX_IMAP_PASSWORD").unwrap_or_default(),
            pgp_operator_fingerprint: parse_env::<String>("FAX_PGP_OPERATOR_FINGERPRINT")
                .unwrap_or_default(),
            poll_interval: Duration::from_secs(
                parse_env::<u64>("FAX_POLL_INTERVAL_SECS").unwrap_or(60),
            ),
        };

        let mfritas = MfritasConfig {
            heartbeat_interval_days: parse_env::<u32>("MFRITAS_HEARTBEAT_INTERVAL_DAYS")
                .unwrap_or(30),
            alert_days: parse_env::<u32>("MFRITAS_ALERT_DAYS").unwrap_or(60),
            disparo_days: parse_env::<u32>("MFRITAS_DISPARO_DAYS").unwrap_or(90),
        };

        let dev_bearer = parse_env::<String>("TRENCHPASS_DEV_BEARER").ok();
        if env.is_production() && dev_bearer.is_some() {
            return Err(Error::Config(
                "TRENCHPASS_DEV_BEARER prohibido en producción".into(),
            ));
        }

        Ok(Self {
            server,
            tls,
            vault,
            database,
            otel,
            proton_pass,
            fax,
            mfritas,
            env,
            dev_bearer,
        })
    }
}

fn required(key: &str) -> Result<String> {
    std::env::var(key).map_err(|_| Error::Config(format!("falta variable obligatoria {key}")))
}

fn parse_env<T: std::str::FromStr>(key: &str) -> Result<T>
where
    T::Err: std::fmt::Display,
{
    let raw = std::env::var(key).map_err(|_| Error::Config(format!("falta {key}")))?;
    raw.parse::<T>()
        .map_err(|e| Error::Config(format!("{key}: {e}")))
}

fn parse_bool(key: &str) -> Result<bool> {
    let raw = std::env::var(key).map_err(|_| Error::Config(format!("falta {key}")))?;
    Ok(matches!(
        raw.to_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    ))
}
