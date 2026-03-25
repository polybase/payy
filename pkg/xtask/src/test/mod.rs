mod changes;
mod graph;
mod metadata;
mod workspace;

use std::collections::BTreeSet;
use std::process::Command;

use clap::Args;
use contextful::ResultContextExt;

use crate::error::{Result, XTaskError, workspace_root};
use crate::git::collect_changed_files;

use crate::test::changes::{ChangedCrates, determine_changed_crates, sorted_list};
use crate::test::graph::DependencyGraph;
use crate::test::metadata::{Metadata, load_metadata};
use crate::test::workspace::{CompiledWorkspace, compile_workspace_tests};

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

    let direct_list = sorted_list(&changed.direct);
    println!("Changed crates: {}", direct_list.join(", "));

    let affected = graph::calculate_affected_crates(graph, &changed.direct)
        .into_iter()
        .collect::<BTreeSet<String>>();
    let additional = affected
        .iter()
        .filter(|crate_name| !changed.direct.contains(*crate_name))
        .cloned()
        .collect::<BTreeSet<String>>();

    if !additional.is_empty() {
        let additional_list = sorted_list(&additional);
        println!(
            "Transitively affected crates: {}",
            additional_list.join(", ")
        );
    }

    let mut execution_order = direct_list.clone();
    execution_order.extend(sorted_list(&additional));
    Some(execution_order)
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

    let metadata = load_metadata(&repo_root)?;
    let changed = determine_changed_crates(&metadata, &repo_root, &changed_files);

    if changed.touches_all {
        println!("Detected root manifest change; all workspace crate tests will run");
    }

    let graph = DependencyGraph::build(&metadata);
    let Some(execution_order) = prepare_execution_order(&graph, &changed) else {
        return Ok(());
    };

    println!("Building workspace tests with `cargo test --workspace --no-run`...");
    let compiled = compile_workspace_tests(&repo_root, &metadata)?;

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
