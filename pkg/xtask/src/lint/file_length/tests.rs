use std::fs;
use std::path::Path;

use tempfile::tempdir;

use super::{
    DEFAULT_LIMIT, FileInspection, collect_rust_files, inspect_file, parse_override_limit,
    report::relative_display,
};

#[test]
fn collects_rust_files_recursively_in_sorted_order() {
    let temp = tempdir().expect("tempdir");
    write_file(temp.path().join("pkg/z.rs"), "");
    write_file(temp.path().join("pkg/a/nested.rs"), "");
    write_file(temp.path().join("pkg/a/ignored.txt"), "");

    let files = collect_rust_files(&temp.path().join("pkg")).expect("collect files");
    let relative = files
        .iter()
        .map(|path| relative_display(temp.path(), path))
        .collect::<Vec<_>>();

    assert_eq!(relative, vec!["pkg/a/nested.rs", "pkg/z.rs"]);
}

#[test]
fn detects_default_limit_violation() {
    let temp = tempdir().expect("tempdir");
    let path = temp.path().join("long.rs");
    write_file(&path, lines(DEFAULT_LIMIT + 1));

    let FileInspection::Checked { line_count, limit } = inspect_file(&path).expect("inspect file")
    else {
        panic!("file should be checked");
    };

    assert_eq!(line_count, DEFAULT_LIMIT + 1);
    assert_eq!(limit, DEFAULT_LIMIT);
}

#[test]
fn honors_override_in_first_twenty_lines() {
    let temp = tempdir().expect("tempdir");
    let path = temp.path().join("overridden.rs");
    let source = format!(
        "  // lint-long-file-override   allow-max-lines = 250\n{}",
        lines(DEFAULT_LIMIT + 1)
    );
    write_file(&path, source);

    let FileInspection::Checked { line_count, limit } = inspect_file(&path).expect("inspect file")
    else {
        panic!("file should be checked");
    };

    assert_eq!(line_count, DEFAULT_LIMIT + 2);
    assert_eq!(limit, 250);
}

#[test]
fn uses_first_override_in_first_twenty_lines() {
    let temp = tempdir().expect("tempdir");
    let path = temp.path().join("multiple_overrides.rs");
    let source = format!(
        "// lint-long-file-override allow-max-lines=250\n// lint-long-file-override allow-max-lines=300\n{}",
        lines(DEFAULT_LIMIT + 1)
    );
    write_file(&path, source);

    let FileInspection::Checked { limit, .. } = inspect_file(&path).expect("inspect file") else {
        panic!("file should be checked");
    };

    assert_eq!(limit, 250);
}

#[test]
fn ignores_override_after_first_twenty_lines() {
    let temp = tempdir().expect("tempdir");
    let path = temp.path().join("late_override.rs");
    let source = format!(
        "{}// lint-long-file-override allow-max-lines=300\n",
        lines(20)
    );
    write_file(&path, source);

    let FileInspection::Checked { limit, .. } = inspect_file(&path).expect("inspect file") else {
        panic!("file should be checked");
    };

    assert_eq!(limit, DEFAULT_LIMIT);
}

#[test]
fn skips_generated_first_line_files() {
    let temp = tempdir().expect("tempdir");
    let path = temp.path().join("generated.rs");
    write_file(
        &path,
        format!("// @generated\n{}", lines(DEFAULT_LIMIT + 1)),
    );

    assert!(matches!(
        inspect_file(&path).expect("inspect file"),
        FileInspection::Generated
    ));
}

#[test]
fn preserves_wc_line_count_semantics() {
    let temp = tempdir().expect("tempdir");
    let path = temp.path().join("partial.rs");
    write_file(&path, "one\ntwo");

    let FileInspection::Checked { line_count, .. } = inspect_file(&path).expect("inspect file")
    else {
        panic!("file should be checked");
    };

    assert_eq!(line_count, 1);
}

#[test]
fn parses_override_limit_like_the_shell_pattern() {
    assert_eq!(
        parse_override_limit(b"\t// lint-long-file-override allow-max-lines = 300"),
        Some(300)
    );
    assert_eq!(
        parse_override_limit(b"// lint-long-file-overrideallow-max-lines=300"),
        None
    );
    assert_eq!(
        parse_override_limit(b"// lint-long-file-override allow-max-lines = nope"),
        None
    );
}

fn lines(count: usize) -> String {
    "line\n".repeat(count)
}

fn write_file(path: impl AsRef<Path>, contents: impl AsRef<[u8]>) {
    let path = path.as_ref();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent dir");
    }
    fs::write(path, contents).expect("write file");
}
