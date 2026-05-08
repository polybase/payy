use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};
use std::time::Instant;

use contextful::ResultContextExt;

use crate::error::{Result, XTaskError};
use crate::lint::steps::{StepResult, run_command};
use crate::setup::ensure_spec_lint;

pub fn run_spec_lint(repo_root: &Path) -> Result<StepResult> {
    let start = Instant::now();
    match ensure_spec_lint(repo_root) {
        Ok(()) => {}
        Err(XTaskError::Io(source)) if source.kind() == ErrorKind::NotFound => {
            return Ok(StepResult::failed(
                "Spec lint",
                "spec-lint install prerequisites are missing".to_owned(),
                start.elapsed(),
            ));
        }
        Err(error @ XTaskError::CommandFailure { program: "npm", .. }) => {
            return Ok(StepResult::failed(
                "Spec lint",
                "npm ci failed while installing spec-lint dependencies".to_owned(),
                start.elapsed(),
            )
            .with_extra_output(vec![format!("{error}")]));
        }
        Err(error) => return Err(error),
    }

    let args = spec_lint_args(repo_root)?;
    let arg_refs = args.iter().map(String::as_str).collect::<Vec<_>>();
    let status = match run_command(repo_root, "node", &arg_refs) {
        Ok(status) => status,
        Err(XTaskError::Io(source)) if source.kind() == ErrorKind::NotFound => {
            return Ok(StepResult::failed(
                "Spec lint",
                "node not found; spec-lint requires Node.js tooling".to_owned(),
                start.elapsed(),
            ));
        }
        Err(error) => return Err(error),
    };

    if status.success() {
        Ok(StepResult::success(
            "Spec lint",
            "Spec files validated successfully".to_owned(),
            start.elapsed(),
        ))
    } else {
        Ok(StepResult::failed(
            "Spec lint",
            "spec-lint reported warnings or errors".to_owned(),
            start.elapsed(),
        ))
    }
}

fn spec_lint_args(repo_root: &Path) -> Result<Vec<String>> {
    let mut args = vec!["docs/tools/spec-lint".to_owned()];
    args.extend(glob_container_entries(repo_root, "docs/specs")?);
    args.extend(glob_container_entries(repo_root, "docs/specs-wip")?);
    Ok(args)
}

fn glob_container_entries(repo_root: &Path, container: &str) -> Result<Vec<String>> {
    let root = repo_root.join(container);
    let mut entries = fs::read_dir(&root)
        .with_context(|| format!("read spec-lint container directory {}", root.display()))?
        .collect::<std::result::Result<Vec<_>, _>>()
        .with_context(|| format!("list spec-lint container directory {}", root.display()))?
        .into_iter()
        .filter_map(|entry| container_entry(entry.path(), repo_root))
        .collect::<Vec<_>>();
    entries.sort();
    Ok(entries)
}

fn container_entry(path: PathBuf, repo_root: &Path) -> Option<String> {
    if path
        .file_name()
        .is_some_and(|name| name.to_string_lossy().starts_with('.'))
    {
        return None;
    }

    if path.is_dir() || is_spec_file(&path) {
        return Some(
            path.strip_prefix(repo_root)
                .ok()?
                .to_string_lossy()
                .to_string(),
        );
    }

    None
}

fn is_spec_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext == "md" || ext == "mdx")
}
