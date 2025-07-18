use std::{
    io::{Read, Write},
    path::PathBuf,
    process::Command,
};

use flate2::{read::GzEncoder, Compression};
use tempfile::{NamedTempFile, TempDir};
use tracing::{error, info};

use super::Backend;

pub struct CliBackend;

impl Backend for CliBackend {
    fn prove(
        program: &[u8],
        _bytecode: &[u8],
        witness: &[u8],
        recursive: bool,
        oracle_hash_keccak: bool,
    ) -> crate::Result<Vec<u8>> {
        let mut witness_gz = GzEncoder::new(witness, Compression::none());
        let mut witness_gz_buf = Vec::with_capacity(witness.len() + 0xFF);
        witness_gz.read_to_end(&mut witness_gz_buf)?;
        let witness_gz = witness_gz_buf;

        let mut program_file = NamedTempFile::with_suffix(".json")?;
        program_file.write_all(program)?;
        program_file.flush()?;

        let mut witness_file = NamedTempFile::new()?;
        witness_file.write_all(&witness_gz)?;
        witness_file.flush()?;

        let output_dir = TempDir::new()?;

        let mut cmd = Command::new(PathBuf::from("bb"));
        cmd.arg("prove")
            .arg("-v")
            .arg("--scheme")
            .arg("ultra_honk")
            .arg("-b")
            .arg(program_file.path())
            .arg("-w")
            .arg(witness_file.path())
            .arg("-o")
            .arg(output_dir.path());

        if recursive {
            cmd.arg("--honk_recursion")
                .arg("1")
                .arg("--init_kzg_accumulator");
        }

        if oracle_hash_keccak {
            cmd.arg("--oracle_hash").arg("keccak");
        }

        let output = cmd.output()?;
        if !output.status.success() {
            let stderr = String::from_utf8(output.stderr)?;
            return Err(stderr.into());
        }

        let proof_path = output_dir.path().join("proof");
        let mut proof = std::fs::read(&proof_path)?;

        // Remove first 4 bytes
        proof.drain(..4);

        Ok(proof)
    }

    fn verify(proof: &[u8], key: &[u8], oracle_hash_keccak: bool) -> crate::Result<()> {
        let mut key_file = NamedTempFile::new()?;
        key_file.write_all(key)?;
        key_file.flush()?;

        let mut proof_file = NamedTempFile::new()?;
        proof_file.write_all(proof)?;
        proof_file.flush()?;

        let mut cmd = Command::new(PathBuf::from("bb"));
        cmd.arg("verify")
            .arg("-v")
            .arg("--scheme")
            .arg("ultra_honk")
            .arg("-k")
            .arg(key_file.path())
            .arg("-p")
            .arg(proof_file.path());

        if oracle_hash_keccak {
            cmd.arg("--oracle_hash").arg("keccak");
        }

        let output = cmd.output()?;
        info!("output {:?}", output);

        if !output.status.success() {
            // TODO: return false instead? maybe pass -v and parse out verified: {0/1}
            let stderr = String::from_utf8(output.stderr)?;
            error!("proof error: {}", stderr);
            return Err(stderr.into());
        }

        Ok(())
    }
}
