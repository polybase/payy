use std::{
    io::{Read, Write},
    path::PathBuf,
};

use async_trait::async_trait;
use barretenberg_interface::{BbBackend, Error, Result};
use flate2::{Compression, read::GzEncoder};
use tempfile::{NamedTempFile, TempDir};
use tokio::process::Command;
use tracing::{error, info};

pub struct CliBackend;

#[async_trait]
impl BbBackend for CliBackend {
    async fn prove(
        &self,
        program: &[u8],
        _bytecode: &[u8],
        key: &[u8],
        witness: &[u8],
        oracle_hash_keccak: bool,
    ) -> Result<Vec<u8>> {
        let mut witness_gz = GzEncoder::new(witness, Compression::none());
        let mut witness_gz_buf = Vec::with_capacity(witness.len() + 0xFF);
        witness_gz
            .read_to_end(&mut witness_gz_buf)
            .map_err(|e| Error::ImplementationSpecific(e.into()))?;
        let witness_gz = witness_gz_buf;

        let mut program_file = NamedTempFile::with_suffix(".json")
            .map_err(|e| Error::ImplementationSpecific(e.into()))?;
        program_file
            .write_all(program)
            .map_err(|e| Error::ImplementationSpecific(e.into()))?;
        program_file
            .flush()
            .map_err(|e| Error::ImplementationSpecific(e.into()))?;

        let mut witness_file =
            NamedTempFile::new().map_err(|e| Error::ImplementationSpecific(e.into()))?;
        witness_file
            .write_all(&witness_gz)
            .map_err(|e| Error::ImplementationSpecific(e.into()))?;
        witness_file
            .flush()
            .map_err(|e| Error::ImplementationSpecific(e.into()))?;

        let mut key_file =
            NamedTempFile::new().map_err(|e| Error::ImplementationSpecific(e.into()))?;
        key_file
            .write_all(key)
            .map_err(|e| Error::ImplementationSpecific(e.into()))?;
        key_file
            .flush()
            .map_err(|e| Error::ImplementationSpecific(e.into()))?;

        let output_dir = TempDir::new().map_err(|e| Error::ImplementationSpecific(e.into()))?;

        let mut cmd = Command::new(PathBuf::from("bb"));
        cmd.kill_on_drop(true);
        cmd.arg("prove")
            .arg("-v")
            .arg("--scheme")
            .arg("ultra_honk")
            .arg("-b")
            .arg(program_file.path())
            .arg("-w")
            .arg(witness_file.path())
            .arg("-k")
            .arg(key_file.path())
            .arg("-o")
            .arg(output_dir.path());

        if oracle_hash_keccak {
            cmd.arg("--oracle_hash").arg("keccak");
        }

        let output = cmd
            .output()
            .await
            .map_err(|e| Error::ImplementationSpecific(e.into()))?;
        if !output.status.success() {
            let stderr = String::from_utf8(output.stderr)
                .map_err(|e| Error::ImplementationSpecific(e.into()))?;
            return Err(Error::Backend(stderr));
        }

        let proof_path = output_dir.path().join("proof");
        let mut proof = tokio::fs::read(&proof_path)
            .await
            .map_err(|e| Error::ImplementationSpecific(e.into()))?;

        let public_inputs_path = output_dir.path().join("public_inputs");
        let public_inputs = tokio::fs::read(&public_inputs_path)
            .await
            .map_err(|e| Error::ImplementationSpecific(e.into()))?;

        proof.splice(0..0, public_inputs);

        Ok(proof)
    }

    async fn verify(
        &self,
        proof: &[u8],
        public_inputs: &[u8],
        key: &[u8],
        oracle_hash_keccak: bool,
    ) -> Result<()> {
        let mut key_file =
            NamedTempFile::new().map_err(|e| Error::ImplementationSpecific(e.into()))?;
        key_file
            .write_all(key)
            .map_err(|e| Error::ImplementationSpecific(e.into()))?;
        key_file
            .flush()
            .map_err(|e| Error::ImplementationSpecific(e.into()))?;

        let mut proof_file =
            NamedTempFile::new().map_err(|e| Error::ImplementationSpecific(e.into()))?;
        proof_file
            .write_all(proof)
            .map_err(|e| Error::ImplementationSpecific(e.into()))?;
        proof_file
            .flush()
            .map_err(|e| Error::ImplementationSpecific(e.into()))?;

        let mut public_inputs_file =
            NamedTempFile::new().map_err(|e| Error::ImplementationSpecific(e.into()))?;
        public_inputs_file
            .write_all(public_inputs)
            .map_err(|e| Error::ImplementationSpecific(e.into()))?;
        public_inputs_file
            .flush()
            .map_err(|e| Error::ImplementationSpecific(e.into()))?;

        let mut cmd = Command::new(PathBuf::from("bb"));
        cmd.kill_on_drop(true);
        cmd.arg("verify")
            .arg("-v")
            .arg("--scheme")
            .arg("ultra_honk")
            .arg("-k")
            .arg(key_file.path())
            .arg("-p")
            .arg(proof_file.path())
            .arg("-i")
            .arg(public_inputs_file.path());

        if oracle_hash_keccak {
            cmd.arg("--oracle_hash").arg("keccak");
        }

        let output = cmd
            .output()
            .await
            .map_err(|e| Error::ImplementationSpecific(e.into()))?;
        info!("output {:?}", output);

        if !output.status.success() {
            let stderr = String::from_utf8(output.stderr)
                .map_err(|e| Error::ImplementationSpecific(e.into()))?;
            error!("proof error: {}", stderr);
            return Err(Error::Backend(stderr));
        }

        Ok(())
    }
}
