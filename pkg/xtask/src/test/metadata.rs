use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::Command;

use contextful::ResultContextExt;
use serde::Deserialize;

use crate::error::{Result, XTaskError};

#[derive(Debug)]
pub struct Metadata {
    packages: HashMap<String, Package>,
    workspace_members: Vec<String>,
    workspace_member_set: HashSet<String>,
    resolve: HashMap<String, Vec<String>>,
}

#[derive(Debug)]
pub struct Package {
    pub name: String,
    manifest_dir_abs: PathBuf,
}

impl Package {
    pub fn manifest_dir_abs(&self) -> &Path {
        &self.manifest_dir_abs
    }
}

impl Metadata {
    pub fn workspace_packages(&self) -> impl Iterator<Item = &Package> {
        self.workspace_members
            .iter()
            .filter_map(|id| self.packages.get(id))
    }

    pub fn package_for_id(&self, id: &str) -> Option<&Package> {
        self.packages.get(id)
    }

    pub fn package_by_name(&self, name: &str) -> Option<&Package> {
        self.packages.values().find(|package| package.name == name)
    }

    pub fn workspace_member_ids(&self) -> &[String] {
        &self.workspace_members
    }

    pub fn is_workspace_member(&self, id: &str) -> bool {
        self.workspace_member_set.contains(id)
    }

    pub fn dependencies_for(&self, id: &str) -> impl Iterator<Item = &String> {
        self.resolve
            .get(id)
            .map(|deps| deps.iter())
            .into_iter()
            .flatten()
    }
}

#[derive(Deserialize)]
struct RawMetadata {
    packages: Vec<RawPackage>,
    workspace_members: Vec<String>,
    resolve: Option<RawResolve>,
}

#[derive(Deserialize)]
struct RawPackage {
    id: String,
    name: String,
    manifest_path: PathBuf,
}

#[derive(Deserialize)]
struct RawResolve {
    nodes: Vec<RawNode>,
}

#[derive(Deserialize)]
struct RawNode {
    id: String,
    dependencies: Vec<String>,
}

pub fn load_metadata(repo_root: &Path) -> Result<Metadata> {
    let output = Command::new("cargo")
        .args(["metadata", "--format-version", "1"])
        .current_dir(repo_root)
        .output()
        .context("spawn cargo metadata command")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(XTaskError::CargoMetadataFailed { stderr });
    }

    let raw = serde_json::from_slice::<RawMetadata>(&output.stdout)
        .context("parse cargo metadata output")?;

    let workspace_member_set = raw
        .workspace_members
        .iter()
        .cloned()
        .collect::<HashSet<String>>();

    let mut packages = HashMap::new();
    for raw_package in raw.packages {
        if !workspace_member_set.contains(&raw_package.id) {
            continue;
        }

        let manifest_dir =
            raw_package
                .manifest_path
                .parent()
                .ok_or_else(|| XTaskError::InvalidCrateManifest {
                    path: raw_package.manifest_path.clone(),
                })?;

        let manifest_dir_abs = manifest_dir.to_path_buf();
        manifest_dir_abs
            .strip_prefix(repo_root)
            .map(|_| ())
            .map_err(|_| XTaskError::InvalidCrateManifest {
                path: manifest_dir_abs.clone(),
            })?;

        packages.insert(
            raw_package.id.clone(),
            Package {
                name: raw_package.name,
                manifest_dir_abs,
            },
        );
    }

    let resolve_nodes = raw
        .resolve
        .map(|resolve| {
            resolve
                .nodes
                .into_iter()
                .map(|node| (node.id, node.dependencies))
                .collect()
        })
        .unwrap_or_default();

    let workspace_members = raw.workspace_members;

    Ok(Metadata {
        packages,
        workspace_members,
        workspace_member_set,
        resolve: resolve_nodes,
    })
}
