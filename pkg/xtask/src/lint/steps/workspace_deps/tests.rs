use super::{Value, check_manifest};
use indoc::indoc;
use std::path::{Path, PathBuf};

fn parse_manifest(source: &str) -> Value {
    toml::from_str(source).expect("valid manifest")
}

#[test]
fn detects_plain_version_dependency() {
    let manifest = parse_manifest(indoc! {r#"
        [dependencies]
        serde = "1.0"
    "#});
    let manifest_path = PathBuf::from("/repo/pkg/foo/Cargo.toml");

    let violations = check_manifest(Path::new("/repo"), &manifest_path, &manifest);
    assert_eq!(violations.len(), 1);
    let violation = &violations[0];
    assert_eq!(violation.dependency_name, "serde");
    assert_eq!(violation.section, "dependencies");
    assert_eq!(violation.current_spec, "\"1.0\"");
    assert_eq!(violation.expected_spec, "serde = { workspace = true }");
}

#[test]
fn preserves_features_in_expected_spec() {
    let manifest = parse_manifest(indoc! {r#"
        [dependencies]
        serde = { version = "1.0", features = ["derive"] }
    "#});
    let manifest_path = PathBuf::from("/repo/pkg/foo/Cargo.toml");

    let violations = check_manifest(Path::new("/repo"), &manifest_path, &manifest);
    assert_eq!(violations.len(), 1);
    let violation = &violations[0];
    assert!(violation.expected_spec.contains(r#"features = ["derive"]"#));
    assert_eq!(
        violation.expected_spec,
        "serde = { workspace = true, features = [\"derive\"] }"
    );
}

#[test]
fn ignores_dependencies_already_using_workspace() {
    let manifest = parse_manifest(indoc! {r#"
        [dependencies]
        serde = { workspace = true, features = ["derive"] }
    "#});
    let manifest_path = PathBuf::from("/repo/pkg/foo/Cargo.toml");

    let violations = check_manifest(Path::new("/repo"), &manifest_path, &manifest);
    assert!(violations.is_empty());
}

#[test]
fn flags_workspace_dependencies_with_forbidden_keys() {
    let manifest = parse_manifest(indoc! {r#"
        [dependencies]
        serde = { workspace = true, version = "1.0" }
    "#});
    let manifest_path = PathBuf::from("/repo/pkg/foo/Cargo.toml");

    let violations = check_manifest(Path::new("/repo"), &manifest_path, &manifest);
    assert_eq!(violations.len(), 1);
    let violation = &violations[0];
    assert_eq!(violation.expected_spec, "serde = { workspace = true }");
}

#[test]
fn converts_path_dependencies() {
    let manifest = parse_manifest(indoc! {r#"
        [dependencies]
        guild-interface = { path = "../guild-interface", features = ["client"] }
    "#});
    let manifest_path = PathBuf::from("/repo/pkg/foo/Cargo.toml");

    let violations = check_manifest(Path::new("/repo"), &manifest_path, &manifest);
    assert_eq!(violations.len(), 1);
    let violation = &violations[0];
    assert_eq!(
        violation.expected_spec,
        "guild-interface = { workspace = true, features = [\"client\"] }"
    );
}
