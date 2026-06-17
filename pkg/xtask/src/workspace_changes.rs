use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};
use std::path::Path;

use crate::cargo_metadata::Metadata;

pub struct ChangedCrates {
    pub direct: BTreeSet<String>,
    pub unmatched: Vec<String>,
    pub touches_all: bool,
}

#[derive(Clone, Copy)]
pub enum RootManifestBehavior {
    TouchesAll,
    TreatAsUnmatched,
}

pub struct AffectedCrates {
    pub direct: BTreeSet<String>,
    pub additional: BTreeSet<String>,
}

impl AffectedCrates {
    pub fn ordered_package_names(&self) -> Vec<String> {
        let mut package_names = sorted_list(&self.direct);
        package_names.extend(sorted_list(&self.additional));
        package_names
    }
}

#[derive(Debug)]
pub struct DependencyGraph {
    reverse: HashMap<String, BTreeSet<String>>,
}

impl DependencyGraph {
    pub fn build(metadata: &Metadata) -> Self {
        let mut reverse = HashMap::<String, BTreeSet<String>>::new();

        for package in metadata.workspace_packages() {
            reverse.entry(package.name.clone()).or_default();

            for dependency_id in package.workspace_dependency_ids() {
                if let Some(dep_package) = metadata.package_for_id(dependency_id) {
                    reverse
                        .entry(dep_package.name.clone())
                        .or_default()
                        .insert(package.name.clone());
                }
            }
        }

        DependencyGraph { reverse }
    }

    pub fn dependents_of(&self, crate_name: &str) -> Option<&BTreeSet<String>> {
        self.reverse.get(crate_name)
    }
}

pub fn determine_changed_crates(
    metadata: &Metadata,
    repo_root: &Path,
    changed_files: &BTreeSet<String>,
    root_manifest_behavior: RootManifestBehavior,
) -> ChangedCrates {
    let mut direct = BTreeSet::new();
    let mut unmatched = Vec::new();
    let mut touches_all = false;

    for path_str in changed_files {
        let path = Path::new(path_str);
        if is_root_manifest(path) {
            match root_manifest_behavior {
                RootManifestBehavior::TouchesAll => {
                    touches_all = true;
                    break;
                }
                RootManifestBehavior::TreatAsUnmatched => {}
            }
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

pub fn calculate_affected_crates(
    graph: &DependencyGraph,
    changed_crates: &BTreeSet<String>,
) -> AffectedCrates {
    let mut visited = changed_crates.iter().cloned().collect::<HashSet<_>>();
    let mut queue = changed_crates.iter().cloned().collect::<VecDeque<_>>();

    while let Some(crate_name) = queue.pop_front() {
        if let Some(dependents) = graph.dependents_of(&crate_name) {
            for dependent in dependents {
                if visited.insert(dependent.clone()) {
                    queue.push_back(dependent.clone());
                }
            }
        }
    }

    let additional = visited
        .into_iter()
        .filter(|crate_name| !changed_crates.contains(crate_name))
        .collect::<BTreeSet<_>>();

    AffectedCrates {
        direct: changed_crates.clone(),
        additional,
    }
}

pub fn sorted_list(set: &BTreeSet<String>) -> Vec<String> {
    set.iter().cloned().collect()
}

fn is_root_manifest(path: &Path) -> bool {
    path.ends_with("Cargo.toml") && path.parent().is_none()
}
