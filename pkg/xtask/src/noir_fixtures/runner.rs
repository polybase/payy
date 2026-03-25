use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use clap::Args;
use contextful::ResultContextExt;
use serde::Deserialize;

use crate::error::{Result, workspace_root};

use super::export::export_circuit_fixtures;
use super::tooling::run_checked;

#[derive(Debug, Clone, Args, Default)]
pub(crate) struct NoirFixturesArgs;

#[derive(Debug)]
pub(super) struct CircuitConfig {
    pub(super) name: String,
    pub(super) package_dir: PathBuf,
    pub(super) recursive: bool,
    pub(super) solidity: bool,
    pub(super) oracle_hash: Option<String>,
    pub(super) hash_updates: Vec<HashUpdateSpec>,
}

#[derive(Debug, Clone, Deserialize)]
pub(super) struct HashUpdateSpec {
    pub(super) path: String,
    pub(super) kind: HashUpdateKind,
    pub(super) name: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum HashUpdateKind {
    GlobalField,
    ConstString,
}

#[derive(Debug, Deserialize)]
struct WorkspaceManifest {
    workspace: WorkspaceSection,
}

#[derive(Debug, Deserialize)]
struct WorkspaceSection {
    members: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct PackageManifest {
    package: PackageSection,
}

#[derive(Debug, Deserialize)]
struct PackageSection {
    name: String,
    #[serde(rename = "type")]
    package_type: String,
    #[serde(default)]
    metadata: PackageMetadata,
}

#[derive(Debug, Default, Deserialize)]
struct PackageMetadata {
    #[serde(default)]
    generate_fixtures: GenerateFixturesMetadata,
}

#[derive(Debug, Default, Deserialize)]
struct GenerateFixturesMetadata {
    #[serde(default)]
    recursive: bool,
    #[serde(default)]
    solidity: bool,
    oracle_hash: Option<String>,
    #[serde(default)]
    hash_updates: Vec<HashUpdateSpec>,
}

#[derive(Debug)]
pub(super) struct VerificationKeyHash {
    pub(super) as_field: String,
    pub(super) as_hex: String,
}

pub(crate) fn run_noir_fixtures(_args: NoirFixturesArgs) -> Result<()> {
    let repo_root = workspace_root()?;
    let noir_root = repo_root.join("noir");
    let fixtures_root = repo_root.join("fixtures/circuits");

    let nargo = env::var("NARGO").unwrap_or_else(|_| "nargo".to_owned());
    let backend = env::var("BACKEND").unwrap_or_else(|_| "bb".to_owned());

    let target_dir = noir_root.join("target");
    if target_dir.exists() {
        fs::remove_dir_all(&target_dir)
            .with_context(|| format!("remove noir target directory {}", target_dir.display()))?;
    }

    run_checked(
        "nargo",
        Command::new(&nargo)
            .arg("compile")
            .arg("--workspace")
            .current_dir(&noir_root),
    )?;

    fs::create_dir_all(&fixtures_root)
        .with_context(|| format!("create fixtures directory {}", fixtures_root.display()))?;

    let circuits = load_circuit_configs(&noir_root)?;

    for circuit in circuits {
        println!("================");
        println!("{}", circuit.name.to_uppercase());
        println!("================");

        export_circuit_fixtures(&repo_root, &noir_root, &fixtures_root, &backend, &circuit)?;
    }

    println!("Successfully exported circuit fixtures to fixtures/circuits");
    Ok(())
}

fn load_circuit_configs(noir_root: &Path) -> Result<Vec<CircuitConfig>> {
    let workspace_manifest_path = noir_root.join("Nargo.toml");
    let workspace_manifest_raw = fs::read_to_string(&workspace_manifest_path)
        .with_context(|| format!("read {}", workspace_manifest_path.display()))?;
    let workspace_manifest = toml::from_str::<WorkspaceManifest>(&workspace_manifest_raw)
        .with_context(|| format!("parse {}", workspace_manifest_path.display()))?;

    let mut circuits = Vec::new();

    for member in workspace_manifest.workspace.members {
        let package_dir = noir_root.join(&member);
        let package_manifest_path = package_dir.join("Nargo.toml");
        let package_manifest_raw = fs::read_to_string(&package_manifest_path)
            .with_context(|| format!("read {}", package_manifest_path.display()))?;
        let package_manifest = toml::from_str::<PackageManifest>(&package_manifest_raw)
            .with_context(|| format!("parse {}", package_manifest_path.display()))?;

        if package_manifest.package.package_type != "bin" {
            continue;
        }

        let metadata = package_manifest.package.metadata.generate_fixtures;

        circuits.push(CircuitConfig {
            name: package_manifest.package.name,
            package_dir,
            recursive: metadata.recursive,
            solidity: metadata.solidity,
            oracle_hash: metadata.oracle_hash,
            hash_updates: metadata.hash_updates,
        });
    }

    Ok(circuits)
}
