use std::sync::{Mutex, Once, OnceLock};

#[cfg(feature = "max_params")]
use std::{env, fs, path::PathBuf};

use async_trait::async_trait;
use barretenberg_interface::{BbBackend, Error, Result};
#[cfg(feature = "max_params")]
use contextful::ResultContextExt;
use lazy_static::lazy_static;

pub struct BindingBackend;

static INIT: Once = Once::new();

lazy_static! {
    static ref BB_MUTEX: Mutex<()> = Mutex::new(());
}

#[cfg(feature = "max_params")]
static G1: OnceLock<&'static [u8]> = OnceLock::new();

#[cfg(feature = "max_params")]
fn params_dir() -> Result<PathBuf> {
    if let Ok(path) = env::var("POLYBASE_PARAMS_DIR") {
        return Ok(PathBuf::from(path));
    }

    let home = env::var("HOME").map_err(|err| Error::ImplementationSpecific(err.into()))?;

    Ok(PathBuf::from(home).join(".polybase/fixtures/params"))
}

#[cfg(feature = "max_params")]
fn load_max_params() -> Result<&'static [u8]> {
    let path = params_dir()?.join("g1.max.dat");
    let bytes = fs::read(&path)
        .context("missing g1.max.dat; run scripts/download-fixtures-params.sh")
        .map_err(|err| Error::ImplementationSpecific(Box::new(err)))?;

    Ok(Box::leak(bytes.into_boxed_slice()))
}

#[cfg(not(feature = "max_params"))]
static G1: &[u8] = include_bytes!("../../../fixtures/params/g1.utxo.dat");

#[cfg(feature = "max_params")]
fn g1_bytes() -> Result<&'static [u8]> {
    if let Some(bytes) = G1.get() {
        return Ok(*bytes);
    }

    let bytes = load_max_params()?;
    let _ = G1.set(bytes);
    Ok(bytes)
}

#[cfg(not(feature = "max_params"))]
fn g1_bytes() -> Result<&'static [u8]> {
    Ok(G1)
}

impl BindingBackend {
    fn load_srs() -> Result<()> {
        let g1 = g1_bytes()?;
        INIT.call_once(|| unsafe {
            bb_rs::barretenberg_api::srs::init_srs(g1, (g1.len() / 64) as u32);
        });
        Ok(())
    }
}

#[async_trait]
impl BbBackend for BindingBackend {
    async fn prove(
        &self,
        _program: &[u8],
        _bytecode: &[u8],
        _key: &[u8],
        _witness: &[u8],
        _oracle_hash_keccak: bool,
    ) -> Result<Vec<u8>> {
        let bytecode = _bytecode.to_vec();
        let witness = _witness.to_vec();
        let key = _key.to_vec();

        tokio::task::spawn_blocking(move || {
            let _guard = BB_MUTEX.lock().unwrap();

            Self::load_srs()?;

            let proof = match _oracle_hash_keccak {
                false => unsafe {
                    bb_rs::barretenberg_api::acir::acir_prove_ultra_honk(&bytecode, &witness, &key)
                },
                true => unsafe {
                    bb_rs::barretenberg_api::acir::acir_prove_ultra_keccak_zk_honk(
                        &bytecode, &witness, &key,
                    )
                },
            };

            Ok(proof)
        })
        .await
        .map_err(|e| Error::ImplementationSpecific(e.into()))?
    }

    async fn verify(
        &self,
        _proof: &[u8],
        _public_inputs: &[u8],
        _key: &[u8],
        _oracle_hash_keccak: bool,
    ) -> Result<()> {
        let proof = [_public_inputs, _proof].concat();
        let key = _key.to_vec();

        tokio::task::spawn_blocking(move || {
            let _guard = BB_MUTEX.lock().unwrap();

            Self::load_srs()?;

            let verified = match _oracle_hash_keccak {
                false => unsafe {
                    bb_rs::barretenberg_api::acir::acir_verify_ultra_honk(&proof, &key)
                },
                true => unsafe {
                    bb_rs::barretenberg_api::acir::acir_verify_ultra_keccak_zk_honk(&proof, &key)
                },
            };

            match verified {
                true => Ok(()),
                false => Err(Error::VerificationFailed),
            }
        })
        .await
        .map_err(|e| Error::ImplementationSpecific(e.into()))?
    }
}
