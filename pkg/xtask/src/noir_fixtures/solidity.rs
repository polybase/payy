use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use contextful::ResultContextExt;
use serde_json::{Value, json};
use solc_tooling::ensure_solc;

use crate::error::{Result, XTaskError};

use super::runner::CircuitConfig;
use super::tooling::run_checked;

pub(super) fn export_solidity_artifacts(
    repo_root: &Path,
    backend: &str,
    circuit: &CircuitConfig,
    key_path: &Path,
) -> Result<()> {
    let solidity_output = repo_root
        .join("eth/noir")
        .join(format!("{}.sol", circuit.name));
    let solidity_output_dir = solidity_output
        .parent()
        .ok_or_else(|| XTaskError::NoirManifest {
            path: solidity_output.clone(),
            reason: "missing parent directory for solidity output path".to_owned(),
        })?;

    fs::create_dir_all(solidity_output_dir)
        .with_context(|| format!("create directory {}", solidity_output_dir.display()))?;

    run_checked(
        "bb",
        Command::new(backend)
            .arg("write_solidity_verifier")
            .arg("--scheme")
            .arg("ultra_honk")
            .arg("-k")
            .arg(key_path)
            .arg("-o")
            .arg(&solidity_output),
    )?;

    let solc_path = ensure_solc().context("ensure pinned solc")?;
    let standard_json_input_path = write_solc_standard_json(repo_root, circuit)?;
    let solc_output = run_checked(
        "solc",
        Command::new(&solc_path)
            .arg("--standard-json")
            .arg(&standard_json_input_path)
            .current_dir(repo_root),
    )?;
    fs::remove_file(&standard_json_input_path)
        .with_context(|| format!("remove {}", standard_json_input_path.display()))?;

    let output_json = serde_json::from_slice::<Value>(&solc_output.stdout)
        .context("parse solc standard-json output")?;

    write_solidity_bytecode_outputs(repo_root, circuit, &output_json)?;
    Ok(())
}

fn write_solc_standard_json(repo_root: &Path, circuit: &CircuitConfig) -> Result<PathBuf> {
    let source_file = format!("eth/noir/{}.sol", circuit.name);
    let source_key = format!("{}.sol", circuit.name);
    let input = json!({
        "language": "Solidity",
        "sources": {
            source_key: {
                "urls": [source_file]
            }
        },
        "settings": {
            "optimizer": { "enabled": true, "runs": 0 },
            "debug": { "revertStrings": "strip" },
            "outputSelection": {
                "*": {
                    "*": ["evm.bytecode", "evm.deployedBytecode"],
                    "": ["id"]
                }
            }
        }
    });

    let path = repo_root
        .join("eth/noir")
        .join(format!("{}_solc_input.json", circuit.name));
    let mut payload = serde_json::to_string_pretty(&input).context("serialize solc input json")?;
    payload.push('\n');
    fs::write(&path, payload).with_context(|| format!("write {}", path.display()))?;
    Ok(path)
}

fn write_solidity_bytecode_outputs(
    repo_root: &Path,
    circuit: &CircuitConfig,
    output: &Value,
) -> Result<()> {
    let contracts = output
        .get("contracts")
        .and_then(Value::as_object)
        .ok_or(XTaskError::NoirSolcMissingSourceKey)?;
    let (_, source_contracts) = contracts
        .iter()
        .next()
        .ok_or(XTaskError::NoirSolcMissingSourceKey)?;

    let honk_bytecode = source_contracts
        .get("HonkVerifier")
        .and_then(|value| value.get("evm"))
        .and_then(|value| value.get("bytecode"))
        .and_then(|value| value.get("object"))
        .and_then(Value::as_str)
        .ok_or(XTaskError::NoirSolcMissingContractBytecode {
            contract: "HonkVerifier",
        })?;

    let lib_bytecode = source_contracts
        .get("ZKTranscriptLib")
        .and_then(|value| value.get("evm"))
        .and_then(|value| value.get("bytecode"))
        .and_then(|value| value.get("object"))
        .and_then(Value::as_str)
        .ok_or(XTaskError::NoirSolcMissingContractBytecode {
            contract: "ZKTranscriptLib",
        })?;

    let linkrefs = source_contracts
        .get("HonkVerifier")
        .and_then(|value| value.get("evm"))
        .and_then(|value| value.get("bytecode"))
        .and_then(|value| value.get("linkReferences"))
        .cloned()
        .ok_or(XTaskError::NoirSolcMissingContractBytecode {
            contract: "HonkVerifier",
        })?;

    let contracts_noir_dir = repo_root.join("eth/contracts/noir");
    fs::create_dir_all(&contracts_noir_dir)
        .with_context(|| format!("create directory {}", contracts_noir_dir.display()))?;

    fs::write(
        contracts_noir_dir.join(format!("{}_HonkVerifier.bin", circuit.name)),
        honk_bytecode,
    )
    .with_context(|| {
        format!(
            "write {}",
            contracts_noir_dir
                .join(format!("{}_HonkVerifier.bin", circuit.name))
                .display()
        )
    })?;

    fs::write(
        contracts_noir_dir.join(format!("{}_ZKTranscriptLib.bin", circuit.name)),
        lib_bytecode,
    )
    .with_context(|| {
        format!(
            "write {}",
            contracts_noir_dir
                .join(format!("{}_ZKTranscriptLib.bin", circuit.name))
                .display()
        )
    })?;

    let mut linkrefs_payload =
        serde_json::to_string_pretty(&linkrefs).context("serialize linkrefs json")?;
    linkrefs_payload.push('\n');
    fs::write(
        contracts_noir_dir.join(format!("{}_HonkVerifier.linkrefs.json", circuit.name)),
        linkrefs_payload,
    )
    .with_context(|| {
        format!(
            "write {}",
            contracts_noir_dir
                .join(format!("{}_HonkVerifier.linkrefs.json", circuit.name))
                .display()
        )
    })?;
    Ok(())
}
