use std::{fs, path::PathBuf};

use contextful::ResultContextExt;
use serde_json::{Value, json};

use crate::{
    apps::{Error, Result},
    runtime::BeamApp,
};

pub fn get(app: &BeamApp, app_id: &str, key: &str) -> Result<Value> {
    let path = path(app, app_id, key)?;
    if !path.exists() {
        return Ok(json!({ "value": null, "exists": false }));
    }
    let value =
        serde_json::from_slice::<Value>(&fs::read(path).context("read beam app storage value")?)
            .context("decode beam app storage value")?;
    Ok(json!({ "value": value, "exists": true }))
}

pub fn set(app: &BeamApp, app_id: &str, key: &str, value: Value) -> Result<Value> {
    let path = path(app, app_id, key)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).context("create beam app storage directory")?;
    }
    fs::write(
        path,
        serde_json::to_vec_pretty(&value).context("encode beam app storage value")?,
    )
    .context("write beam app storage value")?;
    Ok(json!(true))
}

pub fn remove(app: &BeamApp, app_id: &str, key: &str) -> Result<Value> {
    let path = path(app, app_id, key)?;
    if path.exists() {
        fs::remove_file(path).context("remove beam app storage value")?;
    }
    Ok(json!(true))
}

fn path(app: &BeamApp, app_id: &str, key: &str) -> Result<PathBuf> {
    if key.is_empty()
        || key.starts_with('.')
        || !key
            .chars()
            .all(|char| char.is_ascii_alphanumeric() || matches!(char, '-' | '_' | '.'))
    {
        return Err(Error::InvalidHostRequest {
            reason: format!("invalid app storage key {key}"),
        });
    }

    Ok(app
        .paths
        .root
        .join("apps")
        .join("data")
        .join(app_id)
        .join(key))
}
