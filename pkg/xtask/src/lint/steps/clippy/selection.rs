use std::collections::BTreeSet;
use std::path::Path;

use crate::cargo_metadata::load_workspace_metadata;
use crate::error::Result;
use crate::git::collect_changed_files;
use crate::lint::LintMode;
use crate::workspace_changes::{
    DependencyGraph, RootManifestBehavior, calculate_affected_crates, determine_changed_crates,
    sorted_list,
};

pub enum ClippySelection {
    Workspace {
        reason: String,
    },
    Packages {
        direct: BTreeSet<String>,
        additional: BTreeSet<String>,
        packages: Vec<String>,
        unmatched: Vec<String>,
    },
    Skip {
        detail: String,
    },
}

pub fn select_clippy_packages(repo_root: &Path, mode: LintMode) -> Result<ClippySelection> {
    if mode.is_check_only() {
        return Ok(ClippySelection::Workspace {
            reason: "--check runs the full workspace".to_owned(),
        });
    }

    let changed_files = collect_changed_files(repo_root)?;
    if changed_files.is_empty() {
        return Ok(ClippySelection::Skip {
            detail: "No changed files detected".to_owned(),
        });
    }

    if let Some(path) = changed_files
        .iter()
        .find(|path| is_global_clippy_input(Path::new(path)))
    {
        return Ok(ClippySelection::Workspace {
            reason: format!("{path} affects workspace clippy configuration"),
        });
    }

    let metadata = load_workspace_metadata(repo_root)?;
    let changed = determine_changed_crates(
        &metadata,
        repo_root,
        &changed_files,
        RootManifestBehavior::TreatAsUnmatched,
    );

    if changed.direct.is_empty() {
        if changed_files.contains("Cargo.toml") {
            return Ok(ClippySelection::Workspace {
                reason: "root Cargo.toml changed without package changes".to_owned(),
            });
        }

        if changed_files.contains("Cargo.lock") {
            return Ok(ClippySelection::Workspace {
                reason: "Cargo.lock changed without a package change".to_owned(),
            });
        }

        if has_rust_relevant_unmatched(&changed.unmatched) {
            return Ok(ClippySelection::Workspace {
                reason: format!(
                    "Rust-relevant files outside workspace crates changed: {}",
                    changed.unmatched.join(", ")
                ),
            });
        }

        return Ok(ClippySelection::Skip {
            detail: "No changed workspace crates detected".to_owned(),
        });
    }

    let graph = DependencyGraph::build(&metadata);
    let affected = calculate_affected_crates(&graph, &changed.direct);
    let packages = affected.ordered_package_names();

    Ok(ClippySelection::Packages {
        direct: affected.direct,
        additional: affected.additional,
        packages,
        unmatched: changed.unmatched,
    })
}

pub fn build_clippy_args(mode: LintMode, selection: &ClippySelection) -> Option<Vec<String>> {
    let mut args = vec!["clippy".to_owned()];
    if mode.is_check_only() {
        args.push("--locked".to_owned());
    }

    match selection {
        ClippySelection::Workspace { .. } => {
            args.push("--workspace".to_owned());
        }
        ClippySelection::Packages { packages, .. } => {
            for package in packages {
                args.push("--package".to_owned());
                args.push(package.clone());
            }
        }
        ClippySelection::Skip { .. } => return None,
    }

    args.extend([
        "--all-targets".to_owned(),
        "--quiet".to_owned(),
        "--".to_owned(),
        "-D".to_owned(),
        "warnings".to_owned(),
    ]);
    Some(args)
}

pub fn print_selection(selection: &ClippySelection) {
    match selection {
        ClippySelection::Workspace { reason } => {
            println!("Running full workspace clippy: {reason}");
        }
        ClippySelection::Packages {
            direct,
            additional,
            unmatched,
            ..
        } => {
            println!(
                "Changed crates for clippy: {}",
                sorted_list(direct).join(", ")
            );
            if !additional.is_empty() {
                println!(
                    "Transitively affected crates for clippy: {}",
                    sorted_list(additional).join(", ")
                );
            }
            if !unmatched.is_empty() {
                println!(
                    "Non-crate changes did not widen clippy scope: {}",
                    unmatched.join(", ")
                );
            }
        }
        ClippySelection::Skip { .. } => {}
    }
}

pub fn selection_success_detail(selection: &ClippySelection) -> String {
    match selection {
        ClippySelection::Workspace { .. } => "All workspace checks passed".to_owned(),
        ClippySelection::Packages { packages, .. } => {
            format!("All checks passed for {} affected crate(s)", packages.len())
        }
        ClippySelection::Skip { detail } => detail.clone(),
    }
}

pub fn selection_failure_detail(selection: &ClippySelection) -> String {
    match selection {
        ClippySelection::Workspace { .. } => "cargo clippy reported workspace issues".to_owned(),
        ClippySelection::Packages { packages, .. } => {
            format!(
                "cargo clippy reported issues for {} affected crate(s)",
                packages.len()
            )
        }
        ClippySelection::Skip { detail } => detail.clone(),
    }
}

fn is_global_clippy_input(path: &Path) -> bool {
    matches!(
        path.to_str(),
        Some("clippy.toml")
            | Some("rust-toolchain")
            | Some("rust-toolchain.toml")
            | Some(".cargo/config")
            | Some(".cargo/config.toml")
    )
}

fn has_rust_relevant_unmatched(paths: &[String]) -> bool {
    paths.iter().any(|path| {
        let path = Path::new(path);
        path.extension().is_some_and(|extension| extension == "rs")
    })
}
