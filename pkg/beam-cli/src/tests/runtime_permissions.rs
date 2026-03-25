#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

#[cfg(unix)]
use tempfile::TempDir;

#[cfg(unix)]
use crate::{
    chains::load_chains, config::load_config, keystore::load_keystore, runtime::ensure_root_dir,
};

#[cfg(unix)]
#[test]
fn ensure_root_dir_restricts_beam_home_permissions() {
    let temp_dir = TempDir::new().expect("create temp dir");
    let root = temp_dir.path().join(".beam");

    std::fs::create_dir_all(&root).expect("create beam root");
    std::fs::set_permissions(&root, std::fs::Permissions::from_mode(0o755))
        .expect("set insecure beam root permissions");

    ensure_root_dir(&root).expect("ensure beam root");

    assert_eq!(
        std::fs::metadata(&root)
            .expect("beam root metadata")
            .permissions()
            .mode()
            & 0o777,
        0o700
    );
}

#[cfg(unix)]
#[tokio::test]
async fn beam_state_files_use_owner_only_permissions() {
    let temp_dir = TempDir::new().expect("create temp dir");
    let root = temp_dir.path().join(".beam");
    ensure_root_dir(&root).expect("ensure beam root");

    load_config(&root).await.expect("load config store");
    load_chains(&root).await.expect("load chains store");
    load_keystore(&root).await.expect("load keystore store");

    for filename in ["config.json", "chains.json", "wallets.json"] {
        let path = root.join(filename);
        assert_eq!(
            std::fs::metadata(&path)
                .unwrap_or_else(|_| panic!("missing {filename}"))
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
    }
}
