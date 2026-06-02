use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use contextful::ResultContextExt;
use json_store::{FileAccess, InvalidJsonBehavior, JsonStore};

use crate::apps::{
    Error, Result,
    model::{AppLock, AppManifest, AppsState, InstalledApp},
};

pub struct AppCache {
    root: PathBuf,
    state: JsonStore<AppsState>,
}

impl AppCache {
    pub async fn load(beam_root: &Path) -> Result<Self> {
        let root = beam_root.join("apps");
        fs::create_dir_all(root.join("apps")).context("create beam app cache")?;
        fs::create_dir_all(root.join("data")).context("create beam app data directory")?;
        let state = JsonStore::new_with_invalid_json_behavior_and_access(
            &root,
            "apps-state.json",
            InvalidJsonBehavior::Error,
            FileAccess::OwnerOnly,
        )
        .await
        .context("load beam app cache state")?;

        Ok(Self { root, state })
    }

    pub async fn installed(&self) -> AppsState {
        self.state.get().await
    }

    pub async fn install(
        &self,
        manifest: &AppManifest,
        manifest_bytes: &[u8],
        module_bytes: &[u8],
        manifest_sha256: &str,
        module_sha256: &str,
        registry_url: &str,
    ) -> Result<()> {
        let app_dir = self.version_dir(&manifest.id, &manifest.version);
        fs::create_dir_all(&app_dir).context("create beam app version directory")?;
        fs::write(app_dir.join("manifest.json"), manifest_bytes)
            .context("write beam app manifest")?;
        fs::write(app_dir.join("module.wasm"), module_bytes).context("write beam app module")?;
        let lock = AppLock {
            id: manifest.id.clone(),
            version: manifest.version.clone(),
            manifest_sha256: manifest_sha256.to_string(),
            module_sha256: module_sha256.to_string(),
            registry_url: registry_url.to_string(),
            installed_at: now(),
        };
        fs::write(
            app_dir.join("lock.json"),
            serde_json::to_vec_pretty(&lock).context("encode beam app lock")?,
        )
        .context("write beam app lock")?;

        self.state
            .update(|state| {
                state.installed.retain(|app| app.id != manifest.id);
                state.installed.push(InstalledApp {
                    id: manifest.id.clone(),
                    active_version: manifest.version.clone(),
                    manifest_sha256: manifest_sha256.to_string(),
                    module_sha256: module_sha256.to_string(),
                    installed_at: lock.installed_at,
                });
            })
            .await
            .context("persist beam app state")?;

        Ok(())
    }

    pub async fn remove(&self, app_id: &str, purge_data: bool) -> Result<()> {
        self.state
            .update(|state| state.installed.retain(|app| app.id != app_id))
            .await
            .context("persist beam app removal")?;
        let app_dir = self.app_dir(app_id);
        if app_dir.exists() {
            fs::remove_dir_all(app_dir).context("remove beam app artifacts")?;
        }
        if purge_data {
            let data_dir = self.root.join("data").join(app_id);
            if data_dir.exists() {
                fs::remove_dir_all(data_dir).context("remove beam app data")?;
            }
        }

        Ok(())
    }

    pub async fn active_manifest(&self, app_id: &str) -> Result<(InstalledApp, AppManifest)> {
        let state = self.state.get().await;
        let installed = state
            .installed
            .into_iter()
            .find(|app| app.id == app_id)
            .ok_or_else(|| Error::AppNotInstalled {
                app: app_id.to_string(),
            })?;
        let manifest_path = self
            .version_dir(app_id, &installed.active_version)
            .join("manifest.json");
        let manifest = serde_json::from_slice::<AppManifest>(
            &fs::read(manifest_path).context("read installed beam app manifest")?,
        )
        .context("decode installed beam app manifest")?;

        Ok((installed, manifest))
    }

    pub fn module_path(&self, app_id: &str, version: &str) -> PathBuf {
        self.version_dir(app_id, version).join("module.wasm")
    }

    pub fn data_dir(&self, app_id: &str) -> PathBuf {
        self.root.join("data").join(app_id)
    }

    fn app_dir(&self, app_id: &str) -> PathBuf {
        self.root.join("apps").join(app_id)
    }

    fn version_dir(&self, app_id: &str, version: &str) -> PathBuf {
        self.app_dir(app_id).join(version)
    }
}

pub fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default()
}
