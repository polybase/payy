// lint-long-file-override allow-max-lines=300
use std::net::IpAddr;

use semver::Version;
use url::Url;

use crate::apps::{
    Error, Result,
    model::{AppManifest, ChainPermission, RegistryIndex},
};

pub fn validate_index(index: &RegistryIndex, registry_url: &str) -> Result<()> {
    validate_registry_url(registry_url)?;
    for app in &index.apps {
        validate_app_id(&app.id)?;
        for version in &app.versions {
            validate_digest("manifest", &version.manifest_sha256)?;
            validate_digest("module", &version.module_sha256)?;
            validate_registry_artifact_url(registry_url, &version.manifest_url)?;
            validate_registry_artifact_url(registry_url, &version.module_url)?;
            Version::parse(&version.version).map_err(|_| Error::RegistryVersionNotFound {
                app: app.id.clone(),
                version: version.version.clone(),
            })?;
        }
    }

    Ok(())
}

pub fn validate_registry_url(value: &str) -> Result<()> {
    parse_registry_url(value).map(|_| ())
}

pub fn validate_manifest(manifest: &AppManifest) -> Result<()> {
    validate_app_id(&manifest.id)?;
    validate_digest("wasm", &manifest.wasm.sha256)?;
    Version::parse(&manifest.version).map_err(|_| Error::AppVersionMismatch {
        actual: manifest.version.clone(),
        expected: "semver".to_string(),
    })?;
    Version::parse(&manifest.min_beam_version).map_err(|_| Error::UnsupportedBeamVersion {
        app: manifest.id.clone(),
        required: manifest.min_beam_version.clone(),
    })?;

    for command in &manifest.commands {
        validate_command_name(&command.name)?;
        if let Some(docs) = &command.docs {
            for argument in &docs.arguments {
                validate_doc_name(&argument.name)?;
            }
            for option in &docs.options {
                validate_doc_name(&option.name)?;
            }
        }
    }
    if let Some(icon) = &manifest.icon {
        validate_digest("icon", &icon.sha256)?;
        parse_registry_url(&icon.url)?;
    }
    for http in &manifest.permissions.http {
        validate_url(&http.url)?;
    }
    for chain in &manifest.permissions.chains {
        validate_chain_permission(chain)?;
    }

    Ok(())
}

pub fn ensure_beam_version(app: &str, required: &str) -> Result<()> {
    let current =
        Version::parse(env!("CARGO_PKG_VERSION")).map_err(|_| Error::UnsupportedBeamVersion {
            app: app.to_string(),
            required: required.to_string(),
        })?;
    let required = Version::parse(required).map_err(|_| Error::UnsupportedBeamVersion {
        app: app.to_string(),
        required: required.to_string(),
    })?;
    if current < required {
        return Err(Error::UnsupportedBeamVersion {
            app: app.to_string(),
            required: required.to_string(),
        });
    }

    Ok(())
}

fn validate_chain_permission(permission: &ChainPermission) -> Result<()> {
    validate_glob(&permission.chain)?;
    validate_optional_globs(permission.contracts.as_deref())?;
    validate_optional_globs(permission.spenders.as_deref())?;

    if let Some(selectors) = permission.selectors.as_deref() {
        for selector in selectors {
            if selector.contains('*') {
                validate_glob(selector)?;
            } else {
                validate_selector(selector)?;
            }
        }
    }

    Ok(())
}

fn validate_app_id(value: &str) -> Result<()> {
    if value.is_empty()
        || !value
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-')
    {
        return Err(Error::InvalidAppId {
            value: value.to_string(),
        });
    }

    Ok(())
}

fn validate_command_name(value: &str) -> Result<()> {
    if value.is_empty()
        || !value
            .chars()
            .all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-')
    {
        return Err(Error::InvalidCommandName {
            value: value.to_string(),
        });
    }

    Ok(())
}

fn validate_doc_name(value: &str) -> Result<()> {
    if value.is_empty() {
        return Err(Error::InvalidCommandName {
            value: value.to_string(),
        });
    }

    Ok(())
}

fn validate_digest(artifact: &str, digest: &str) -> Result<()> {
    let Some(hex) = digest.strip_prefix("sha256:") else {
        return Err(Error::InvalidDigest {
            artifact: artifact.to_string(),
            digest: digest.to_string(),
        });
    };
    if hex.len() != 64 || !hex.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return Err(Error::InvalidDigest {
            artifact: artifact.to_string(),
            digest: digest.to_string(),
        });
    }

    Ok(())
}

fn validate_url(value: &str) -> Result<()> {
    if value.contains('*') {
        validate_glob(value)?;
        return Ok(());
    }

    let url = Url::parse(value).map_err(|_| Error::InvalidPermissionUrl {
        value: value.to_string(),
    })?;
    if url.scheme() != "https" {
        return Err(Error::InvalidPermissionUrl {
            value: value.to_string(),
        });
    }
    if is_blocked_host(url.host_str()) {
        return Err(Error::InvalidPermissionUrl {
            value: value.to_string(),
        });
    }

    Ok(())
}

fn validate_registry_artifact_url(registry_url: &str, value: &str) -> Result<()> {
    let registry = parse_registry_url(registry_url)?;
    let url = parse_registry_url(value)?;
    let prefix = format!("{}/", registry.as_str().trim_end_matches('/'));

    if !same_origin(&registry, &url) || !value.starts_with(&prefix) {
        return Err(Error::InvalidRegistryUrl {
            value: value.to_string(),
        });
    }

    Ok(())
}

fn parse_registry_url(value: &str) -> Result<Url> {
    let url = Url::parse(value).map_err(|_| Error::InvalidRegistryUrl {
        value: value.to_string(),
    })?;
    if url.query().is_some() || url.fragment().is_some() {
        return Err(Error::InvalidRegistryUrl {
            value: value.to_string(),
        });
    }

    match url.scheme() {
        "https" if !is_blocked_host(url.host_str()) => Ok(url),
        "http" if is_loopback_host(url.host_str()) => Ok(url),
        _ => Err(Error::InvalidRegistryUrl {
            value: value.to_string(),
        }),
    }
}

fn same_origin(left: &Url, right: &Url) -> bool {
    left.scheme() == right.scheme()
        && left.host_str() == right.host_str()
        && left.port_or_known_default() == right.port_or_known_default()
}

fn is_blocked_host(host: Option<&str>) -> bool {
    let Some(host) = host else {
        return true;
    };
    if host.eq_ignore_ascii_case("localhost") {
        return true;
    }
    match host.parse::<IpAddr>() {
        Ok(IpAddr::V4(ip)) => {
            ip.is_loopback() || ip.is_private() || ip.is_link_local() || ip.is_unspecified()
        }
        Ok(IpAddr::V6(ip)) => ip.is_loopback() || ip.is_unspecified() || ip.is_unique_local(),
        Err(_) => false,
    }
}

fn is_loopback_host(host: Option<&str>) -> bool {
    let Some(host) = host else {
        return false;
    };
    if host.eq_ignore_ascii_case("localhost") {
        return true;
    }
    match host.parse::<IpAddr>() {
        Ok(IpAddr::V4(ip)) => ip.is_loopback(),
        Ok(IpAddr::V6(ip)) => ip.is_loopback(),
        Err(_) => false,
    }
}

fn validate_optional_globs(values: Option<&[String]>) -> Result<()> {
    if let Some(values) = values {
        for value in values {
            validate_glob(value)?;
        }
    }

    Ok(())
}

fn validate_glob(value: &str) -> Result<()> {
    if value.is_empty() || value.contains("**") {
        return Err(Error::InvalidPermissionGlob {
            value: value.to_string(),
        });
    }

    Ok(())
}

fn validate_selector(value: &str) -> Result<()> {
    let normalized = value.strip_prefix("0x").unwrap_or(value);
    if normalized.len() != 8 || !normalized.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return Err(Error::InvalidPermissionSelector {
            value: value.to_string(),
        });
    }

    Ok(())
}
