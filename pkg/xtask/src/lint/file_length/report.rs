use std::path::{Path, PathBuf};

pub struct Violation {
    pub path: PathBuf,
    pub line_count: usize,
    pub limit: usize,
}

pub fn render_failure(repo_root: &Path, violations: &[Violation]) -> Vec<String> {
    let mut lines = Vec::new();
    lines.push(
        "File length check failed. The following files exceed their configured limits:".to_owned(),
    );
    lines.push(String::new());

    for violation in violations {
        lines.push(format!(
            "- {} has {} lines (limit {})",
            relative_display(repo_root, &violation.path),
            violation.line_count,
            violation.limit
        ));
    }

    lines.push(String::new());
    lines.push("Primary hint: Refactor large files to reduce their length.".to_owned());
    lines.push(
        "Secondary hint: If the additional length is justified, add an override comment at the top of the file."
            .to_owned(),
    );
    lines.push(
        "  Example override comment: '// lint-long-file-override allow-max-lines=300' to bump the limit to 300 lines"
            .to_owned(),
    );
    lines.push("  Bump the limits in increments of 100.".to_owned());
    lines
}

pub fn relative_display(repo_root: &Path, path: &Path) -> String {
    path.strip_prefix(repo_root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}
