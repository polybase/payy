mod workspace;

use std::process::Command;

use clap::Args;
use contextful::ResultContextExt;

use crate::error::{Result, XTaskError, workspace_root};
use crate::git::collect_changed_files;

use crate::cargo_metadata::{Metadata, load_workspace_metadata};
use crate::test::workspace::{CompiledWorkspace, compile_package_tests};
use crate::workspace_changes::{
    ChangedCrates, DependencyGraph, RootManifestBehavior, calculate_affected_crates,
    determine_changed_crates, sorted_list,
};

fn prepare_execution_order(
    graph: &DependencyGraph,
    changed: &ChangedCrates,
) -> Option<Vec<String>> {
    if changed.direct.is_empty() {
        println!("No workspace crates with changes detected; skipping tests");
        if !changed.unmatched.is_empty() {
            print_unmatched_notice(&changed.unmatched);
        }
        return None;
    }

    let affected = calculate_affected_crates(graph, &changed.direct);
    let direct_list = sorted_list(&affected.direct);
    println!("Changed crates: {}", direct_list.join(", "));

    if !affected.additional.is_empty() {
        let additional_list = sorted_list(&affected.additional);
        println!(
            "Transitively affected crates: {}",
            additional_list.join(", ")
        );
    }

    Some(affected.ordered_package_names())
}

fn run_execution_order(
    execution_order: &[String],
    metadata: &Metadata,
    compiled: &CompiledWorkspace,
) -> Result<Vec<String>> {
    let mut failed_crates = Vec::new();

    for crate_name in execution_order {
        let Some(package) = metadata.package_by_name(crate_name) else {
            println!("Skipping crate {crate_name}; unable to locate in cargo metadata");
            continue;
        };

        let binaries = compiled.binaries_for(crate_name);
        if binaries.is_empty() {
            println!("No tests discovered for crate {crate_name}; skipping execution");
            continue;
        }

        let mut crate_failed = false;
        for binary in binaries {
            println!(
                "Running tests for crate {crate_name} target {}...",
                binary.target_name
            );
            let mut command = Command::new(&binary.executable);
            command
                .current_dir(package.manifest_dir_abs())
                .env("CARGO_MANIFEST_DIR", package.manifest_dir_abs())
                .env("CARGO_PRIMARY_PACKAGE", "1");

            if let Some(bin_envs) = compiled.bin_envs(crate_name) {
                for (env_key, path) in bin_envs {
                    command.env(env_key, path);
                }
            }

            let status = command.status().with_context(|| {
                format!(
                    "spawn compiled test binary for crate {crate_name} target {}",
                    binary.target_name
                )
            })?;

            if !status.success() {
                crate_failed = true;
                break;
            }
        }

        if crate_failed {
            failed_crates.push(crate_name.clone());
        }
    }

    Ok(failed_crates)
}

fn print_unmatched_notice(unmatched: &[String]) {
    println!(
        "Notice: changed files outside workspace crates detected: {}",
        unmatched.join(", ")
    );
    println!(
        "These files are ignored by xtask test; run additional tests if Rust code depends on them."
    );
}

#[derive(Args, Default)]
pub struct TestArgs {}

pub fn run_test(_args: TestArgs) -> Result<()> {
    let repo_root = workspace_root()?;
    println!("Running xtask test...");

    let changed_files = collect_changed_files(&repo_root)?;
    if changed_files.is_empty() {
        println!("No changes detected; skipping tests");
        return Ok(());
    }

    let metadata = load_workspace_metadata(&repo_root)?;
    let changed = determine_changed_crates(
        &metadata,
        &repo_root,
        &changed_files,
        RootManifestBehavior::TouchesAll,
    );

    if changed.touches_all {
        println!("Detected root manifest change; all workspace crate tests will run");
    }

    let graph = DependencyGraph::build(&metadata);
    let Some(execution_order) = prepare_execution_order(&graph, &changed) else {
        return Ok(());
    };

    println!("Building targeted tests with `cargo test --no-run -p ...`...");
    let compiled = compile_package_tests(&repo_root, &metadata, &execution_order)?;

    let failed_crates = run_execution_order(&execution_order, &metadata, &compiled)?;

    if !changed.unmatched.is_empty() {
        print_unmatched_notice(&changed.unmatched);
    }

    if failed_crates.is_empty() {
        println!("All targeted tests passed");
        Ok(())
    } else {
        println!("Tests failed for crates: {}", failed_crates.join(", "));
        Err(XTaskError::TestsFailed { failed_crates })
    }
}
