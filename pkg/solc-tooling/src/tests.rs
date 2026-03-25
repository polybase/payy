use super::{Error, ExecutionCheck, SOLC_SHA256_LINUX, SOLC_SHA256_MACOS, SolcTarget};

#[test]
fn detects_linux_x86_64_target() {
    let target = SolcTarget::detect_from("linux", "x86_64").expect("detect linux x86_64 target");

    assert_eq!(target.platform, "linux-amd64");
    assert_eq!(target.filename, "solc-linux-amd64-v0.8.29+commit.ab55807c");
    assert_eq!(target.target_name, "solc-v0.8.29-linux");
    assert_eq!(target.expected_sha256, SOLC_SHA256_LINUX);
}

#[test]
fn detects_macos_x86_64_target() {
    let target = SolcTarget::detect_from("macos", "x86_64").expect("detect macos x86_64 target");

    assert_eq!(target.platform, "macosx-amd64");
    assert_eq!(target.filename, "solc-macosx-amd64-v0.8.29+commit.ab55807c");
    assert_eq!(target.target_name, "solc-v0.8.29-macos");
    assert_eq!(target.expected_sha256, SOLC_SHA256_MACOS);
    assert_eq!(target.execution_check, ExecutionCheck::None);
}

#[test]
fn detects_macos_arm64_target_with_rosetta_probe() {
    let target = SolcTarget::detect_from("macos", "aarch64").expect("detect macos arm64 target");

    assert_eq!(target.platform, "macosx-amd64");
    assert_eq!(target.filename, "solc-macosx-amd64-v0.8.29+commit.ab55807c");
    assert_eq!(target.target_name, "solc-v0.8.29-macos");
    assert_eq!(target.expected_sha256, SOLC_SHA256_MACOS);
    assert_eq!(target.execution_check, ExecutionCheck::Rosetta);
}

#[test]
fn rejects_linux_arm64_with_emulation_guidance() {
    let err = match SolcTarget::detect_from("linux", "aarch64") {
        Ok(_) => panic!("linux arm64 should be unsupported"),
        Err(err) => err,
    };

    assert!(matches!(
        err,
        Error::UnsupportedPlatform {
            os: "linux",
            arch: "aarch64"
        }
    ));
    assert!(err.to_string().contains("linux/amd64 container"));
}
