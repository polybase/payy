use std::fs;
use std::path::Path;
use std::process::Command;

use contextful::ResultContextExt;

use crate::error::{Result, XTaskError};

use super::hash_updates::apply_hash_updates;
use super::runner::{CircuitConfig, VerificationKeyHash};
use super::solidity::export_solidity_artifacts;
use super::tooling::run_checked;

pub(super) fn export_circuit_fixtures(
    repo_root: &Path,
    noir_root: &Path,
    fixtures_root: &Path,
    backend: &str,
    circuit: &CircuitConfig,
) -> Result<()> {
    let circuit_dir = fixtures_root.join(&circuit.name);
    let program_path = circuit_dir.join("program.json");
    let key_path = circuit_dir.join("key");
    let key_fields_path = circuit_dir.join("key_fields.json");

    fs::create_dir_all(&circuit_dir)
        .with_context(|| format!("create circuit directory {}", circuit_dir.display()))?;

    let compiled_program = noir_root
        .join("target")
        .join(format!("{}.json", circuit.name));
    fs::copy(&compiled_program, &program_path).with_context(|| {
        format!(
            "copy compiled program {} to {}",
            compiled_program.display(),
            program_path.display()
        )
    })?;

    write_verification_key(backend, circuit, &program_path, &circuit_dir)?;

    let raw_vk_path = circuit_dir.join("vk");
    if key_path.exists() {
        fs::remove_file(&key_path).with_context(|| format!("remove {}", key_path.display()))?;
    }
    fs::rename(&raw_vk_path, &key_path).with_context(|| {
        format!(
            "move generated verification key {} to {}",
            raw_vk_path.display(),
            key_path.display()
        )
    })?;

    let vk_hash_path = circuit_dir.join("vk_hash");
    if vk_hash_path.exists() {
        fs::remove_file(&vk_hash_path)
            .with_context(|| format!("remove {}", vk_hash_path.display()))?;
    }

    write_key_fields_json(&key_path, &key_fields_path)?;

    let verification_key_hash = run_vk_hash(repo_root, &key_fields_path, &circuit.name)?;
    println!("Verification key hash for {}:", circuit.name);
    println!("  u256: {}", verification_key_hash.as_field);
    println!("  hex: {}", verification_key_hash.as_hex);
    println!();

    apply_hash_updates(circuit, &verification_key_hash)?;

    if circuit.solidity {
        export_solidity_artifacts(repo_root, backend, circuit, &key_path)?;
    }

    Ok(())
}

fn write_verification_key(
    backend: &str,
    circuit: &CircuitConfig,
    program_path: &Path,
    circuit_dir: &Path,
) -> Result<()> {
    let mut command = Command::new(backend);
    command
        .arg("write_vk")
        .arg("--scheme")
        .arg("ultra_honk")
        .arg("-b")
        .arg(program_path)
        .arg("-o")
        .arg(circuit_dir);

    let selected_oracle_hash = circuit
        .oracle_hash
        .as_deref()
        .or({
            if circuit.solidity {
                Some("keccak")
            } else {
                None
            }
        })
        .or({
            if circuit.recursive {
                Some("poseidon2")
            } else {
                None
            }
        });

    if let Some(oracle_hash) = selected_oracle_hash {
        command.arg("--oracle_hash").arg(oracle_hash);
    }

    run_checked("bb", &mut command)?;
    Ok(())
}

fn write_key_fields_json(key_path: &Path, key_fields_path: &Path) -> Result<()> {
    let key_bytes = fs::read(key_path).with_context(|| format!("read {}", key_path.display()))?;
    let fields = key_bytes
        .chunks(32)
        .map(|chunk| format!("0x{}", hex::encode(chunk)))
        .collect::<Vec<_>>();

    let mut output =
        serde_json::to_string_pretty(&fields).context("serialize verification key fields")?;
    output.push('\n');

    fs::write(key_fields_path, output)
        .with_context(|| format!("write {}", key_fields_path.display()))?;
    Ok(())
}

fn run_vk_hash(
    repo_root: &Path,
    key_fields_path: &Path,
    circuit_name: &str,
) -> Result<VerificationKeyHash> {
    let output = run_checked(
        "cargo",
        Command::new("cargo")
            .arg("run")
            .arg("--bin")
            .arg("vk_hash")
            .arg("--")
            .arg(key_fields_path)
            .current_dir(repo_root),
    )?;

    let stdout = String::from_utf8(output.stdout).context("parse vk_hash stdout as utf-8")?;

    let as_field = stdout
        .lines()
        .find_map(|line| line.strip_prefix("u256:").map(str::trim))
        .map(str::to_owned);
    let as_hex = stdout
        .lines()
        .find_map(|line| line.strip_prefix("hex:").map(str::trim))
        .map(str::to_owned);

    match (as_field, as_hex) {
        (Some(as_field), Some(as_hex)) => Ok(VerificationKeyHash { as_field, as_hex }),
        _ => Err(XTaskError::NoirVkHashOutput {
            circuit: circuit_name.to_owned(),
        }),
    }
}
