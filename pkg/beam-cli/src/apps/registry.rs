// lint-long-file-override allow-max-lines=300
use contextful::ResultContextExt;
use reqwest::Client;
use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};

use crate::apps::{
    Error, Result,
    model::{AppManifest, RegistryApp, RegistryIndex, RegistrySignature, RegistryVersion},
    runtime::validate_wasm_module_bytes,
    validate::{ensure_beam_version, validate_index, validate_manifest, validate_registry_url},
};

pub const DEFAULT_REGISTRY_URL: &str = "https://registry.beam.payy.network";

pub async fn fetch_index(registry_url: &str) -> Result<RegistryIndex> {
    validate_registry_url(registry_url)?;
    let url = format!("{}/index.json", registry_url.trim_end_matches('/'));
    let bytes = Client::new()
        .get(&url)
        .send()
        .await
        .context("fetch beam app registry index")?
        .error_for_status()
        .context("validate beam app registry index response")?
        .bytes()
        .await
        .context("read beam app registry index")?;
    let index = serde_json::from_slice::<RegistryIndex>(&bytes)
        .context("decode beam app registry index")?;
    validate_index(&index, registry_url)?;
    verify_signature("index", &index, &index.signature)?;
    Ok(index)
}

pub async fn fetch_manifest(version: &RegistryVersion) -> Result<(AppManifest, Vec<u8>)> {
    let bytes = fetch_bytes(&version.manifest_url).await?;
    ensure_digest("manifest", &bytes, &version.manifest_sha256)?;
    let manifest =
        serde_json::from_slice::<AppManifest>(&bytes).context("decode beam app manifest")?;
    validate_manifest(&manifest)?;
    ensure_beam_version(&manifest.id, &manifest.min_beam_version)?;
    verify_signature("manifest", &manifest, &manifest.signature)?;
    Ok((manifest, bytes))
}

pub async fn fetch_module(version: &RegistryVersion, manifest: &AppManifest) -> Result<Vec<u8>> {
    let bytes = fetch_bytes(&version.module_url).await?;
    ensure_digest("module", &bytes, &version.module_sha256)?;
    ensure_digest("wasm", &bytes, &manifest.wasm.sha256)?;
    validate_wasm_module_bytes(&manifest.id, &manifest.wasm.entrypoint, &bytes)?;
    Ok(bytes)
}

pub fn select_app<'a>(index: &'a RegistryIndex, app_id: &str) -> Result<&'a RegistryApp> {
    index
        .apps
        .iter()
        .find(|app| app.id == app_id)
        .ok_or_else(|| Error::RegistryAppNotFound {
            app: app_id.to_string(),
        })
}

pub fn select_version<'a>(
    app: &'a RegistryApp,
    requested: Option<&str>,
) -> Result<&'a RegistryVersion> {
    match requested {
        Some(version) => app
            .versions
            .iter()
            .find(|candidate| candidate.version == version)
            .ok_or_else(|| Error::RegistryVersionNotFound {
                app: app.id.clone(),
                version: version.to_string(),
            }),
        None => app
            .versions
            .iter()
            .max_by_key(|candidate| semver_key(&candidate.version))
            .ok_or_else(|| Error::RegistryAppNotFound {
                app: app.id.clone(),
            }),
    }
}

pub fn ensure_manifest_matches(
    app_id: &str,
    version: &RegistryVersion,
    manifest: &AppManifest,
) -> Result<()> {
    if manifest.id != app_id {
        return Err(Error::AppIdMismatch {
            actual: manifest.id.clone(),
            expected: app_id.to_string(),
        });
    }
    if manifest.version != version.version {
        return Err(Error::AppVersionMismatch {
            actual: manifest.version.clone(),
            expected: version.version.clone(),
        });
    }

    Ok(())
}

pub fn ensure_digest(artifact: &str, bytes: &[u8], expected: &str) -> Result<()> {
    let actual = format!("sha256:{}", hex::encode(Sha256::digest(bytes)));
    if actual != expected {
        return Err(Error::DigestMismatch {
            actual,
            artifact: artifact.to_string(),
            expected: expected.to_string(),
        });
    }

    Ok(())
}

pub fn registry_url_from_env() -> String {
    std::env::var("BEAM_APP_REGISTRY_URL").unwrap_or_else(|_| DEFAULT_REGISTRY_URL.to_string())
}

pub fn signing_digest<T: Serialize>(value: &T) -> Result<String> {
    let mut payload =
        serde_json::to_value(value).context("encode beam app signing payload value")?;
    blank_top_level_signature(&mut payload);
    sort_json_value(&mut payload);
    let canonical = serde_json::to_vec(&payload).context("encode beam app signing payload")?;
    Ok(format!("sha256:{}", hex::encode(Sha256::digest(canonical))))
}

fn blank_top_level_signature(value: &mut Value) {
    let Some(signature) = value
        .as_object_mut()
        .and_then(|object| object.get_mut("signature"))
        .and_then(Value::as_object_mut)
    else {
        return;
    };
    signature.insert("value".to_string(), Value::String(String::new()));
}

fn sort_json_value(value: &mut Value) {
    match value {
        Value::Array(values) => {
            for value in values {
                sort_json_value(value);
            }
        }
        Value::Object(object) => {
            for value in object.values_mut() {
                sort_json_value(value);
            }
            let mut entries = object
                .iter()
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect::<Vec<_>>();
            entries.sort_by(|left, right| left.0.cmp(&right.0));
            object.clear();
            for (key, value) in entries {
                object.insert(key, value);
            }
        }
        _ => {}
    }
}

fn verify_signature<T: Serialize>(
    artifact: &str,
    value: &T,
    signature: &RegistrySignature,
) -> Result<()> {
    if signature.algorithm != "sha256-dev" {
        return Err(Error::SignatureMismatch {
            artifact: artifact.to_string(),
        });
    }

    let expected = signing_digest(value)?;
    if signature.value != expected {
        return Err(Error::SignatureMismatch {
            artifact: artifact.to_string(),
        });
    }

    Ok(())
}

async fn fetch_bytes(url: &str) -> Result<Vec<u8>> {
    Ok(Client::new()
        .get(url)
        .send()
        .await
        .context("fetch beam app artifact")?
        .error_for_status()
        .context("validate beam app artifact response")?
        .bytes()
        .await
        .context("read beam app artifact")?
        .to_vec())
}

fn semver_key(version: &str) -> semver::Version {
    semver::Version::parse(version).unwrap_or_else(|_| semver::Version::new(0, 0, 0))
}
