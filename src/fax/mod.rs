//! Vía-fax · canal out-of-band PGP-firmado para comandos críticos (ADR-0007).
//!
//! Flujo:
//! 1. Worker IMAP polea Proton cada `FaxConfig::poll_interval`.
//! 2. Cada mensaje UNSEEN: parsea MIME, extrae cuerpo armored OpenPGP.
//! 3. Verifica firma con `sequoia-openpgp` contra el fingerprint del operador.
//! 4. Comprueba nonce + timestamp (replay protection vía `ReplayCache`).
//! 5. Parsea YAML payload → [`FaxCommand`] → [`dispatch::execute`].
//! 6. Marca el mensaje como `\Seen` sólo si todo cuadra (los inválidos quedan
//!    UNSEEN para forensia).

pub mod commands;
pub mod dispatch;
pub mod imap;
pub mod pgp;

pub use commands::{FaxCommand, FaxEnvelope};
pub use pgp::VerifiedPayload;

use std::sync::Arc;

use tokio::task::JoinHandle;
use tracing::{error, info, warn};

use crate::AppState;

/// Lanza el worker IMAP. Devuelve el `JoinHandle` para que `main.rs` pueda
/// hacer `abort()` en shutdown si quisiera.
///
/// Si la config está incompleta (`imap_host` o `imap_user` o
/// `pgp_operator_fingerprint` vacíos), devuelve `None` y loguea `warn!` ·
/// no es error duro porque vía-fax es opcional para arrancar.
pub fn spawn_fax_worker(state: Arc<AppState>) -> Option<JoinHandle<()>> {
    let cfg = &state.config.fax;
    if cfg.imap_host.is_empty()
        || cfg.imap_user.is_empty()
        || cfg.pgp_operator_fingerprint.is_empty()
    {
        warn!(
            target: "fax.worker",
            host_set = !cfg.imap_host.is_empty(),
            user_set = !cfg.imap_user.is_empty(),
            fpr_set = !cfg.pgp_operator_fingerprint.is_empty(),
            "vía-fax inactivo · config incompleta (FAX_IMAP_HOST / FAX_IMAP_USER / FAX_PGP_OPERATOR_FINGERPRINT)"
        );
        return None;
    }
    info!(
        target: "fax.worker",
        host = %cfg.imap_host,
        port = cfg.imap_port,
        user = %cfg.imap_user,
        fpr = %cfg.pgp_operator_fingerprint,
        poll_interval = ?cfg.poll_interval,
        "vía-fax worker arrancando"
    );
    Some(tokio::spawn(async move {
        if let Err(e) = imap::run(state).await {
            error!(target: "fax.worker", error = %e, "vía-fax worker terminado con error · supervisor reiniciar");
        }
    }))
}

/// Errores de cualquier etapa del pipeline vía-fax.
#[derive(Debug, thiserror::Error)]
pub enum FaxError {
    #[error("PGP: {0}")]
    Pgp(String),
    #[error("comando inválido: {0}")]
    Command(String),
    #[error("IMAP: {0}")]
    Imap(String),
    #[error("dispatch: {0}")]
    Dispatch(String),
    #[error("replay: nonce ya visto o timestamp fuera de ventana")]
    Replay,
    #[error("MIME: {0}")]
    Mime(String),
    #[error("operator cert no cargado o fingerprint distinto")]
    NoOperatorCert,
}

pub type FaxResult<T> = Result<T, FaxError>;
