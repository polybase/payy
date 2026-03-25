use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;

use contextful::ResultContextExt;
use serde::Deserialize;

use crate::error::{Result, XTaskError};

use crate::test::metadata::Metadata;

pub struct CompiledWorkspace {
    test_binaries: HashMap<String, Vec<TestBinary>>,
    bin_executables: HashMap<String, HashMap<String, PathBuf>>,
}

impl CompiledWorkspace {
    pub fn binaries_for(&self, crate_name: &str) -> Vec<TestBinary> {
        self.test_binaries
            .get(crate_name)
            .cloned()
            .unwrap_or_default()
    }

    pub fn bin_envs(&self, crate_name: &str) -> Option<&HashMap<String, PathBuf>> {
        self.bin_executables.get(crate_name)
    }
}

#[derive(Clone)]
pub struct TestBinary {
    pub target_name: String,
    pub executable: PathBuf,
}

#[derive(Deserialize)]
struct CargoMessage {
    reason: String,
    #[serde(default)]
    package_id: Option<String>,
    #[serde(default)]
    target: Option<CargoTarget>,
    #[serde(default)]
    profile: Option<CargoProfile>,
    #[serde(default)]
    executable: Option<PathBuf>,
}

#[derive(Deserialize)]
struct CargoTarget {
    name: String,
    kind: Vec<String>,
}

#[derive(Deserialize)]
struct CargoProfile {
    #[serde(default)]
    test: bool,
}

pub fn compile_workspace_tests(repo_root: &Path, metadata: &Metadata) -> Result<CompiledWorkspace> {
    let output = Command::new("cargo")
        .arg("test")
        .arg("--workspace")
        .arg("--message-format=json-render-diagnostics")
        .arg("--no-run")
        .arg("--quiet")
        .current_dir(repo_root)
        .output()
        .context("spawn cargo test --workspace --no-run")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        return Err(XTaskError::CommandFailure {
            program: "cargo",
            status: output.status.code(),
            stderr,
        });
    }

    let stdout = String::from_utf8(output.stdout).context("convert cargo test output to utf-8")?;

    let mut test_binaries = HashMap::<String, Vec<TestBinary>>::new();
    let mut bin_executables = HashMap::<String, HashMap<String, PathBuf>>::new();

    for line in stdout.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let message = serde_json::from_str::<CargoMessage>(trimmed)
            .with_context(|| format!("parse cargo message: {trimmed}"))?;

        if message.reason != "compiler-artifact" {
            continue;
        }

        let package_id = match message.package_id {
            Some(id) => id,
            None => continue,
        };

        let target = match message.target {
            Some(target) => target,
            None => continue,
        };

        let Some(package) = metadata.package_for_id(&package_id) else {
            continue;
        };

        if let Some(executable) = message.executable.clone()
            && target.kind.iter().any(|kind| kind == "bin")
        {
            bin_executables
                .entry(package.name.clone())
                .or_default()
                .insert(
                    format!("CARGO_BIN_EXE_{}", sanitize_bin_name(&target.name)),
                    executable.clone(),
                );
        }

        let profile = match message.profile {
            Some(profile) => profile,
            None => continue,
        };

        if !profile.test {
            continue;
        }

        let executable = match message.executable {
            Some(path) => path,
            None => continue,
        };

        test_binaries
            .entry(package.name.clone())
            .or_default()
            .push(TestBinary {
                target_name: target.name,
                executable,
            });
    }

    Ok(CompiledWorkspace {
        test_binaries,
        bin_executables,
    })
}

fn sanitize_bin_name(name: &str) -> String {
    name.replace('-', "_")
}
