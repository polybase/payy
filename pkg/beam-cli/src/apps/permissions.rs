use crate::apps::{
    Error, Result,
    model::{AppPermissions, ChainOperation, ChainPermission},
};

pub fn ensure_chain_scope(
    permissions: &AppPermissions,
    chain: &str,
    operation: ChainOperation,
    target: Option<&str>,
    selector: Option<&str>,
    spender: Option<&str>,
) -> Result<()> {
    let scope = chain_scope(permissions, chain, &operation)?;
    if let Some(target) = target {
        ensure_optional_scope(scope.contracts.as_deref(), target).map_err(|_| {
            Error::ContractPermissionDenied {
                target: target.to_string(),
            }
        })?;
    }
    if let Some(selector) = selector {
        ensure_optional_scope(scope.selectors.as_deref(), selector).map_err(|_| {
            Error::SelectorPermissionDenied {
                selector: selector.to_string(),
            }
        })?;
    }
    if let Some(spender) = spender {
        ensure_optional_scope(scope.spenders.as_deref(), spender).map_err(|_| {
            Error::SpenderPermissionDenied {
                spender: spender.to_string(),
            }
        })?;
    }

    Ok(())
}

pub fn glob_matches(pattern: &str, value: &str) -> bool {
    let pattern = pattern.to_ascii_lowercase();
    let value = value.to_ascii_lowercase();
    if pattern == "*" {
        return true;
    }
    match pattern.split_once('*') {
        Some((prefix, suffix)) => value.starts_with(prefix) && value.ends_with(suffix),
        None => pattern == value,
    }
}

fn chain_scope<'a>(
    permissions: &'a AppPermissions,
    chain: &str,
    operation: &ChainOperation,
) -> Result<&'a ChainPermission> {
    permissions
        .chains
        .iter()
        .find(|permission| {
            glob_matches(&permission.chain, chain)
                && permission
                    .operations
                    .iter()
                    .any(|candidate| candidate == operation)
        })
        .ok_or_else(|| Error::ChainPermissionDenied {
            chain: chain.to_string(),
            operation: format!("{operation:?}"),
        })
}

fn ensure_optional_scope(patterns: Option<&[String]>, value: &str) -> std::result::Result<(), ()> {
    match patterns {
        Some(patterns) if patterns.iter().any(|pattern| glob_matches(pattern, value)) => Ok(()),
        Some(_) => Err(()),
        None => Ok(()),
    }
}
