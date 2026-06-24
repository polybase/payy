use std::{fs, io::Read, path::Path};

use contextful::ResultContextExt;
use contracts::Secp256k1SecretKey;
use rand::RngCore;

use crate::{
    cli::PrivateKeySourceArgs,
    error::{Error, Result},
    keystore::{prompt_private_key, wallet_address},
    output::CommandOutput,
    runtime::BeamApp,
};

pub(crate) async fn show_address(
    app: &BeamApp,
    private_key_source: &PrivateKeySourceArgs,
) -> Result<()> {
    let secret_key = load_secret_key(private_key_source)?;
    let address = format!("{:#x}", wallet_address(&secret_key)?);
    CommandOutput::new(
        address.clone(),
        serde_json::json!({
            "address": address,
        }),
    )
    .compact(address)
    .print(app.output_mode)
}

pub(crate) fn load_secret_key(private_key_source: &PrivateKeySourceArgs) -> Result<Vec<u8>> {
    let private_key = read_private_key(private_key_source)?;
    let secret_key = parse_secret_key(&private_key)?;
    Ok(secret_key)
}

fn read_private_key(private_key_source: &PrivateKeySourceArgs) -> Result<String> {
    if private_key_source.private_key_stdin {
        return read_private_key_from_stdin();
    }

    if let Some(fd) = private_key_source.private_key_fd {
        return read_private_key_from_fd(fd);
    }

    prompt_private_key()
}

fn read_private_key_from_stdin() -> Result<String> {
    let mut private_key = String::new();
    std::io::stdin()
        .read_to_string(&mut private_key)
        .context("read beam private key from stdin")?;
    Ok(private_key)
}

pub(crate) fn read_private_key_from_fd(fd: u32) -> Result<String> {
    let path = Path::new("/dev/fd").join(fd.to_string());
    Ok(fs::read_to_string(path).context("read beam private key from file descriptor")?)
}

pub(crate) fn parse_secret_key(private_key: &str) -> Result<Vec<u8>> {
    let decoded = hex::decode(private_key.trim().trim_start_matches("0x"))
        .map_err(|_| Error::InvalidPrivateKey)?;
    validate_secret_key(&decoded)?;
    Ok(decoded)
}

pub(crate) fn validate_secret_key(secret_key: &[u8]) -> Result<()> {
    let _ = Secp256k1SecretKey::from_slice(secret_key).map_err(|_| Error::InvalidPrivateKey)?;
    Ok(())
}

pub(crate) fn generate_secret_key() -> [u8; 32] {
    loop {
        let mut secret_key = [0u8; 32];
        rand::thread_rng().fill_bytes(&mut secret_key);
        if Secp256k1SecretKey::from_slice(&secret_key).is_ok() {
            return secret_key;
        }
    }
}
