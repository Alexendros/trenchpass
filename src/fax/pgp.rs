//! Verificación PGP de comandos vía-fax.
//!
//! Espera mensajes armored OpenPGP (`-----BEGIN PGP MESSAGE-----`) producidos
//! por `gpg --sign --armor command.yaml`. La firma es del operador cuyo
//! fingerprint primario (40 hex chars) coincide con
//! [`crate::config::FaxConfig::pgp_operator_fingerprint`].
//!
//! Política:
//! - Cert válido `now()` por [`StandardPolicy`] (no expirado, no revocado).
//! - Una firma "good" del primario o subkey vinculado al cert esperado.
//! - El payload limpio (`literal data`) se devuelve para parseo YAML.

use std::io::Read;

use sequoia_openpgp::cert::Cert;
use sequoia_openpgp::parse::stream::{
    MessageLayer, MessageStructure, VerificationHelper, VerifierBuilder,
};
use sequoia_openpgp::parse::Parse;
use sequoia_openpgp::policy::StandardPolicy;
use sequoia_openpgp::{Fingerprint, KeyHandle};

use super::{FaxError, FaxResult};

/// Payload limpio (sin armadura PGP) tras verificación correcta.
#[derive(Debug, Clone)]
pub struct VerifiedPayload {
    pub body: Vec<u8>,
    pub signer_fpr: Fingerprint,
}

/// Verifica un mensaje OpenPGP armored firmado.
///
/// - `armored_msg`: bytes del mensaje (suele venir como `\-\-\-\-\-BEGIN PGP MESSAGE-\-\-\-\-\-` ...).
/// - `expected_fpr`: fingerprint hex del cert autorizado (40 chars, sin espacios ni `0x`).
/// - `operator_cert`: cert público importado al gateway (fuente de truth para verificar).
pub fn verify_armored(
    armored_msg: &[u8],
    expected_fpr: &str,
    operator_cert: &Cert,
) -> FaxResult<VerifiedPayload> {
    let expected: Fingerprint = expected_fpr
        .parse()
        .map_err(|e| FaxError::Pgp(format!("expected_fpr parse: {e}")))?;

    if operator_cert.fingerprint() != expected {
        return Err(FaxError::Pgp(format!(
            "operator_cert fpr ({}) ≠ expected ({})",
            operator_cert.fingerprint(),
            expected
        )));
    }

    let policy = StandardPolicy::new();
    let helper = Helper {
        cert: operator_cert.clone(),
        expected: expected.clone(),
    };
    let mut verifier = VerifierBuilder::from_bytes(armored_msg)
        .map_err(|e| FaxError::Pgp(format!("VerifierBuilder::from_bytes: {e}")))?
        .with_policy(&policy, None, helper)
        .map_err(|e| FaxError::Pgp(format!("with_policy: {e}")))?;

    let mut body = Vec::new();
    verifier
        .read_to_end(&mut body)
        .map_err(|e| FaxError::Pgp(format!("read body: {e}")))?;

    Ok(VerifiedPayload {
        body,
        signer_fpr: expected,
    })
}

/// `VerificationHelper` que sólo acepta firmas del cert configurado.
struct Helper {
    cert: Cert,
    expected: Fingerprint,
}

impl VerificationHelper for Helper {
    fn get_certs(&mut self, _ids: &[KeyHandle]) -> sequoia_openpgp::Result<Vec<Cert>> {
        Ok(vec![self.cert.clone()])
    }

    fn check(&mut self, structure: MessageStructure) -> sequoia_openpgp::Result<()> {
        for layer in structure.into_iter() {
            if let MessageLayer::SignatureGroup { results } = layer {
                for good in results.into_iter().flatten() {
                    if good.ka.cert().fingerprint() == self.expected {
                        return Ok(());
                    }
                }
            }
        }
        anyhow::bail!("ninguna firma válida del fingerprint esperado")
    }
}

#[cfg(test)]
pub(crate) mod test_support {
    //! Helpers para generar certs y firmar mensajes en tests sin GPG externo.
    use sequoia_openpgp::cert::CertBuilder;
    use sequoia_openpgp::policy::StandardPolicy;
    use sequoia_openpgp::serialize::stream::{Armorer, LiteralWriter, Message, Signer};
    use sequoia_openpgp::Cert;
    use std::io::Write;

    /// Genera un cert sólo-firma con un userid sintético.
    pub fn gen_test_cert() -> Cert {
        let (cert, _rev) = CertBuilder::new()
            .add_userid("Operator <op@example.test>")
            .add_signing_subkey()
            .generate()
            .expect("CertBuilder::generate");
        cert
    }

    /// Produce `gpg --sign --armor` equivalente: cert + mensaje → bytes armored.
    pub fn sign_armored(cert: &Cert, payload: &[u8]) -> Vec<u8> {
        let policy = StandardPolicy::new();
        let keypair = cert
            .keys()
            .with_policy(&policy, None)
            .secret()
            .alive()
            .revoked(false)
            .for_signing()
            .next()
            .expect("signing key amalgamation")
            .key()
            .clone()
            .into_keypair()
            .expect("into_keypair");

        let mut sink = Vec::new();
        {
            let message = Message::new(&mut sink);
            let message = Armorer::new(message).build().expect("armorer");
            let signer = Signer::new(message, keypair)
                .expect("signer new")
                .build()
                .expect("signer build");
            let mut lit = LiteralWriter::new(signer).build().expect("literal writer");
            lit.write_all(payload).expect("write payload");
            lit.finalize().expect("finalize");
        }
        sink
    }
}

#[cfg(test)]
mod tests {
    use super::test_support::*;
    use super::*;

    #[test]
    fn verify_ok_returns_payload() {
        let cert = gen_test_cert();
        let payload = b"command: invalidate-all\nnonce: x\ntimestamp: 1\n";
        let armored = sign_armored(&cert, payload);
        let fpr = format!("{}", cert.fingerprint());
        let verified = verify_armored(&armored, &fpr, &cert).expect("verify");
        assert_eq!(verified.body, payload);
        assert_eq!(format!("{}", verified.signer_fpr), fpr);
    }

    #[test]
    fn verify_rejects_wrong_fingerprint() {
        let cert = gen_test_cert();
        let other = gen_test_cert();
        let payload = b"command: invalidate-all\n";
        let armored = sign_armored(&cert, payload);
        let other_fpr = format!("{}", other.fingerprint());
        let err = verify_armored(&armored, &other_fpr, &other).expect_err("must fail");
        assert!(matches!(err, FaxError::Pgp(_)));
    }

    #[test]
    fn verify_rejects_when_passed_cert_differs_from_expected() {
        let signer = gen_test_cert();
        let other = gen_test_cert();
        let payload = b"command: invalidate-all\n";
        let armored = sign_armored(&signer, payload);
        let signer_fpr = format!("{}", signer.fingerprint());
        let err = verify_armored(&armored, &signer_fpr, &other).expect_err("cert mismatch");
        assert!(matches!(err, FaxError::Pgp(_)));
    }

    #[test]
    fn verify_rejects_garbage() {
        let cert = gen_test_cert();
        let fpr = format!("{}", cert.fingerprint());
        let err = verify_armored(b"not a pgp message", &fpr, &cert).expect_err("garbage");
        assert!(matches!(err, FaxError::Pgp(_)));
    }
}
