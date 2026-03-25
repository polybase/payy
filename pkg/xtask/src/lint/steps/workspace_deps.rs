// lint-long-file-override allow-max-lines=260
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use contextful::ResultContextExt;
use toml::Value;

use crate::error::Result;

use crate::lint::steps::StepResult;

const DEPENDENCY_TABLE_KEYS: &[&str] = &["dependencies", "dev-dependencies", "build-dependencies"];
const FORBIDDEN_KEYS: &[&str] = &["version", "path", "git", "branch", "tag", "rev"];

pub fn run_workspace_deps(repo_root: &Path) -> Result<StepResult> {
    let start = Instant::now();
    let manifests = collect_package_manifests(repo_root)?;
    let mut violations = Vec::new();

    for manifest_path in manifests {
        let content = fs::read_to_string(&manifest_path)
            .with_context(|| format!("read {}", manifest_path.display()))?;
        let manifest = toml::from_str::<Value>(&content)
            .with_context(|| format!("parse {}", manifest_path.display()))?;

        violations.extend(check_manifest(repo_root, &manifest_path, &manifest));
    }

    if violations.is_empty() {
        return Ok(StepResult::success(
            "Workspace dependencies",
            "All crate dependencies inherit from the workspace".to_string(),
            start.elapsed(),
        ));
    }

    let mut grouped = BTreeMap::<String, Vec<Violation>>::new();
    for violation in violations {
        let key = violation.manifest_display.clone();
        grouped.entry(key).or_default().push(violation);
    }

    let total = grouped.values().map(|items| items.len()).sum::<usize>();

    let mut extra = Vec::new();
    for (manifest, mut items) in grouped {
        items.sort_by(|a, b| a.dependency_name.cmp(&b.dependency_name));

        let mut lines = Vec::with_capacity(items.len() * 2 + 1);
        lines.push(format!("{manifest}:"));
        for item in items {
            lines.push(format!(
                "  - {} ({}): specified as {}",
                item.dependency_name, item.section, item.current_spec
            ));
            lines.push(format!("    Expected: {}", item.expected_spec));
        }
        extra.push(lines.join("\n"));
    }

    let summary = format!("{total} dependencies not using workspace inheritance");

    Ok(
        StepResult::failed("Workspace dependencies", summary, start.elapsed())
            .with_extra_output(extra),
    )
}

fn collect_package_manifests(repo_root: &Path) -> Result<Vec<PathBuf>> {
    let mut manifests = Vec::new();
    let pkg_dir = repo_root.join("pkg");
    let entries = fs::read_dir(&pkg_dir).with_context(|| format!("list {}", pkg_dir.display()))?;

    for entry in entries {
        let entry = entry.with_context(|| format!("iterate {}", pkg_dir.display()))?;
        let metadata = entry
            .metadata()
            .with_context(|| format!("load metadata for {}", entry.path().display()))?;
        if !metadata.is_dir() {
            continue;
        }

        if entry.file_name() == "workspace-hack" {
            continue;
        }

        let manifest_path = entry.path().join("Cargo.toml");
        if manifest_path.exists() {
            manifests.push(manifest_path);
        }
    }

    manifests.sort();
    Ok(manifests)
}

fn check_manifest(repo_root: &Path, manifest_path: &Path, manifest: &Value) -> Vec<Violation> {
    let mut violations = Vec::new();

    for section in DEPENDENCY_TABLE_KEYS {
        let Some(table) = manifest.get(*section).and_then(Value::as_table) else {
            continue;
        };

        for (dependency, value) in table {
            if dependency == "workspace-hack" {
                continue;
            }

            if dependency_uses_workspace(value) {
                continue;
            }

            violations.push(Violation::new(
                repo_root,
                manifest_path,
                dependency,
                section,
                value,
            ));
        }
    }

    violations
}

fn dependency_uses_workspace(value: &Value) -> bool {
    match value {
        Value::Table(table) => table_uses_workspace(table),
        _ => false,
    }
}

fn table_uses_workspace(table: &toml::value::Table) -> bool {
    table
        .get("workspace")
        .and_then(Value::as_bool)
        .filter(|value| *value)
        .filter(|_| !table_contains_forbidden(table))
        .is_some()
}

fn table_contains_forbidden(table: &toml::value::Table) -> bool {
    table.iter().any(|(key, value)| {
        FORBIDDEN_KEYS.contains(&key.as_str())
            || matches!(value, Value::Table(nested) if table_contains_forbidden(nested))
    })
}

fn expected_spec(name: &str, value: &Value) -> String {
    let mut entries = Vec::new();
    entries.push(("workspace".to_string(), Value::Boolean(true)));
    entries.extend(clone_allowed_entries(value));
    format!("{name} = {{ {} }}", render_entries(&entries))
}

fn format_current_spec(value: &Value) -> String {
    match value {
        Value::Table(_) => format!("{{ {} }}", render_entries(&clone_all_entries(value))),
        Value::String(_) => value.to_string(),
        _ => value.to_string(),
    }
}

fn clone_allowed_entries(value: &Value) -> Vec<(String, Value)> {
    match value {
        Value::Table(table) => table
            .iter()
            .filter(|(key, _)| *key != "workspace" && !FORBIDDEN_KEYS.contains(&key.as_str()))
            .map(|(key, candidate)| (key.clone(), candidate.clone()))
            .collect(),
        _ => Vec::new(),
    }
}

fn clone_all_entries(value: &Value) -> Vec<(String, Value)> {
    match value {
        Value::Table(table) => table
            .iter()
            .map(|(key, candidate)| (key.clone(), candidate.clone()))
            .collect(),
        _ => Vec::new(),
    }
}

fn render_entries(entries: &[(String, Value)]) -> String {
    entries
        .iter()
        .map(|(key, value)| format!("{key} = {}", value_to_string(value)))
        .collect::<Vec<_>>()
        .join(", ")
}

fn value_to_string(value: &Value) -> String {
    match value {
        Value::Table(table) => {
            let nested = table
                .iter()
                .map(|(key, val)| format!("{key} = {}", value_to_string(val)))
                .collect::<Vec<_>>()
                .join(", ");
            format!("{{ {nested} }}")
        }
        _ => value.to_string(),
    }
}

struct Violation {
    manifest_display: String,
    dependency_name: String,
    section: String,
    current_spec: String,
    expected_spec: String,
}

impl Violation {
    fn new(
        repo_root: &Path,
        manifest_path: &Path,
        dependency: &str,
        section: &str,
        value: &Value,
    ) -> Self {
        let manifest_display = manifest_path
            .strip_prefix(repo_root)
            .map(|path| path.to_string_lossy().replace('\\', "/"))
            .unwrap_or_else(|_| manifest_path.display().to_string());

        Violation {
            manifest_display,
            dependency_name: dependency.to_string(),
            section: section.to_string(),
            current_spec: format_current_spec(value),
            expected_spec: expected_spec(dependency, value),
        }
    }
}

#[cfg(test)]
mod tests;
