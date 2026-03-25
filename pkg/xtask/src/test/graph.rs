use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};

use crate::test::metadata::Metadata;

#[derive(Debug)]
pub struct DependencyGraph {
    reverse: HashMap<String, BTreeSet<String>>,
}

impl DependencyGraph {
    pub fn build(metadata: &Metadata) -> Self {
        let mut reverse = HashMap::<String, BTreeSet<String>>::new();

        for member_id in metadata.workspace_member_ids() {
            if let Some(package) = metadata.package_for_id(member_id) {
                reverse.entry(package.name.clone()).or_default();

                for dependency_id in metadata.dependencies_for(member_id) {
                    if !metadata.is_workspace_member(dependency_id) {
                        continue;
                    }

                    if let Some(dep_package) = metadata.package_for_id(dependency_id) {
                        reverse
                            .entry(dep_package.name.clone())
                            .or_default()
                            .insert(package.name.clone());
                    }
                }
            }
        }

        DependencyGraph { reverse }
    }

    pub fn dependents_of(&self, crate_name: &str) -> Option<&BTreeSet<String>> {
        self.reverse.get(crate_name)
    }
}

pub fn calculate_affected_crates(
    graph: &DependencyGraph,
    changed_crates: &BTreeSet<String>,
) -> BTreeSet<String> {
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

    visited.into_iter().collect()
}
