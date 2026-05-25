//! Smoke offline del pipeline vía-fax (PR7): cert sintético → firma armored
//! → MIME → verify → parse → dispatch.
//!
//! No toca red ni Vault real. Verifica que el código del módulo `fax`
//! compone una vuelta completa con datos generados in-memory.
//!
//! Uso:
//! ```bash
//! cargo run --example fax_smoke
//! ```

use anyhow::Result;
use sequoia_openpgp::cert::CertBuilder;
use sequoia_openpgp::policy::StandardPolicy;
use sequoia_openpgp::serialize::stream::{Armorer, LiteralWriter, Message, Signer};
use std::io::Write;
use trenchpass::fax::commands::{parse_envelope, FaxCommand};
use trenchpass::fax::pgp::verify_armored;

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "trenchpass=debug,info".parse().unwrap()),
        )
        .with_target(true)
        .init();

    println!("[fax_smoke] generando cert sintético");
    let (cert, _rev) = CertBuilder::new()
        .add_userid("Operator <op@example.test>")
        .add_signing_subkey()
        .generate()?;
    let fpr = format!("{}", cert.fingerprint());
    println!("[fax_smoke] fingerprint = {fpr}");

    let payload = b"nonce: 6f5d3e26-cb04-4b78-bbac-3a3c8b4f0001\n\
                    timestamp: 1748160000\n\
                    command: invalidate\n\
                    path: kv/notion/api_key\n";

    println!("[fax_smoke] firmando payload");
    let policy = StandardPolicy::new();
    let keypair = cert
        .keys()
        .with_policy(&policy, None)
        .secret()
        .alive()
        .revoked(false)
        .for_signing()
        .next()
        .expect("signing key")
        .key()
        .clone()
        .into_keypair()?;
    let mut armored = Vec::new();
    {
        let message = Message::new(&mut armored);
        let message = Armorer::new(message).build()?;
        let signer = Signer::new(message, keypair).build()?;
        let mut lit = LiteralWriter::new(signer).build()?;
        lit.write_all(payload)?;
        lit.finalize()?;
    }
    println!(
        "[fax_smoke] armored OpenPGP message generado ({} bytes)",
        armored.len()
    );

    println!("[fax_smoke] verificando firma");
    let verified = verify_armored(&armored, &fpr, &cert)?;
    println!(
        "[fax_smoke] firma OK · signer={} · payload={} bytes",
        verified.signer_fpr,
        verified.body.len()
    );

    let envelope = parse_envelope(&verified.body)?;
    println!(
        "[fax_smoke] envelope parseado · timestamp={} · command={:?}",
        envelope.timestamp, envelope.command
    );
    match envelope.command {
        FaxCommand::Invalidate { path } => {
            println!("[fax_smoke] dispatch invalidate path={path}");
            println!("[fax_smoke] (en runtime real: state.vault.invalidate(&path))");
        }
        _ => unreachable!("payload tests this branch"),
    }

    println!("[fax_smoke] FIN · pipeline OK");
    Ok(())
}
