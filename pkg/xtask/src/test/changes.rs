use std::collections::BTreeSet;
use std::path::Path;

use crate::test::metadata::Metadata;

pub struct ChangedCrates {
    pub direct: BTreeSet<String>,
    pub unmatched: Vec<String>,
    pub touches_all: bool,
}

pub fn determine_changed_crates(
    metadata: &Metadata,
    repo_root: &Path,
    changed_files: &BTreeSet<String>,
) -> ChangedCrates {
    let mut direct = BTreeSet::new();
    let mut unmatched = Vec::new();
    let mut touches_all = false;

    for path_str in changed_files {
        let path = Path::new(path_str);
        if touches_workspace_manifest(path) {
            touches_all = true;
            break;
        }

        let absolute_path = repo_root.join(path);
        let mut matched = false;

        for package in metadata.workspace_packages() {
            if absolute_path.starts_with(package.manifest_dir_abs()) {
                direct.insert(package.name.clone());
                matched = true;
            }
        }

        if !matched {
            unmatched.push(path_str.clone());
        }
    }

    if touches_all {
        direct = metadata
            .workspace_packages()
            .map(|package| package.name.clone())
            .collect();
        unmatched.clear();
    }

    ChangedCrates {
        direct,
        unmatched,
        touches_all,
    }
}

pub fn sorted_list(set: &BTreeSet<String>) -> Vec<String> {
    set.iter().cloned().collect()
}

fn touches_workspace_manifest(path: &Path) -> bool {
    if path.ends_with("Cargo.toml") && path.parent().is_none() {
        return true;
    }

    if path.ends_with("Cargo.lock") {
        // A small change like adding a feature or dependency to a single crate
        // can update the Cargo.lock for the whole workspace,
        // but there is no point in running all tests in that case.
        // So this is purposely commented out.
        // return true;
    }

    false
}
