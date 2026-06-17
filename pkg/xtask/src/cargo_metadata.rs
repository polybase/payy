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
}

#[derive(Debug)]
pub struct Package {
    pub name: String,
    manifest_dir_abs: PathBuf,
    manifest_path_abs: PathBuf,
    workspace_dependency_ids: Vec<String>,
}

impl Package {
    pub fn manifest_dir_abs(&self) -> &Path {
        &self.manifest_dir_abs
    }

    pub fn manifest_path_abs(&self) -> &Path {
        &self.manifest_path_abs
    }

    pub fn workspace_dependency_ids(&self) -> &[String] {
        &self.workspace_dependency_ids
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
}

#[derive(Deserialize)]
struct RawMetadata {
    packages: Vec<RawPackage>,
    workspace_members: Vec<String>,
}

#[derive(Deserialize)]
struct RawPackage {
    id: String,
    name: String,
    manifest_path: PathBuf,
    #[serde(default)]
    dependencies: Vec<RawDependency>,
}

#[derive(Deserialize)]
struct RawDependency {
    path: Option<PathBuf>,
}

struct WorkspaceRawPackage {
    id: String,
    name: String,
    manifest_dir_abs: PathBuf,
    manifest_path_abs: PathBuf,
    dependencies: Vec<RawDependency>,
}

pub fn load_workspace_metadata(repo_root: &Path) -> Result<Metadata> {
    let mut command = Command::new("cargo");
    command.args(["metadata", "--format-version", "1", "--no-deps"]);

    let output = command
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

    let mut raw_workspace_packages = Vec::new();
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

        raw_workspace_packages.push(WorkspaceRawPackage {
            id: raw_package.id,
            name: raw_package.name,
            manifest_dir_abs,
            manifest_path_abs: raw_package.manifest_path,
            dependencies: raw_package.dependencies,
        });
    }

    let package_id_by_manifest_dir = raw_workspace_packages
        .iter()
        .map(|package| (package.manifest_dir_abs.clone(), package.id.clone()))
        .collect::<HashMap<_, _>>();

    let mut packages = HashMap::new();
    for raw_package in raw_workspace_packages {
        let workspace_dependency_ids = raw_package
            .dependencies
            .iter()
            .filter_map(|dependency| dependency.path.as_ref())
            .filter_map(|path| package_id_by_manifest_dir.get(path))
            .cloned()
            .collect::<Vec<_>>();

        packages.insert(
            raw_package.id.clone(),
            Package {
                name: raw_package.name,
                manifest_dir_abs: raw_package.manifest_dir_abs,
                manifest_path_abs: raw_package.manifest_path_abs,
                workspace_dependency_ids,
            },
        );
    }

    let workspace_members = raw.workspace_members;

    Ok(Metadata {
        packages,
        workspace_members,
    })
}
