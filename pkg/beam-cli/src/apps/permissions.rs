use crate::apps::{
    Error, Result,
    model::{AppPermissions, ChainOperation, ChainPermission, DynamicContractScope},
};

pub fn ensure_chain_scope_with_dynamic(
    permissions: &AppPermissions,
    dynamic_contracts: &[DynamicContractScope],
    chain: &str,
    operation: ChainOperation,
    target: Option<&str>,
    selector: Option<&str>,
    spender: Option<&str>,
) -> Result<()> {
    let scope = chain_scope(permissions, chain, &operation)?;
    if let Some(target) = target {
        ensure_optional_contract_scope(
            scope.contracts.as_deref(),
            dynamic_contracts,
            chain,
            target,
        )
        .map_err(|_| Error::ContractPermissionDenied {
            target: target.to_string(),
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

pub fn normalize_dynamic_contracts(
    dynamic_contracts: &[DynamicContractScope],
) -> Vec<DynamicContractScope> {
    let mut out = Vec::new();
    for scope in dynamic_contracts {
        let normalized_contract = scope.contract.to_ascii_lowercase();
        if out.iter().any(|existing: &DynamicContractScope| {
            glob_matches(&existing.chain, &scope.chain)
                && existing.contract.eq_ignore_ascii_case(&normalized_contract)
        }) {
            continue;
        }
        out.push(DynamicContractScope {
            chain: scope.chain.clone(),
            contract: normalized_contract,
            reason: scope.reason.clone(),
        });
    }

    out
}

pub fn validate_dynamic_contracts(
    dynamic_contracts: &[DynamicContractScope],
    chain: &str,
) -> Result<()> {
    for scope in dynamic_contracts {
        if !glob_matches(&scope.chain, chain) {
            return Err(Error::ChainPermissionDenied {
                chain: scope.chain.clone(),
                operation: "dynamic-contract".to_string(),
            });
        }
        if scope.contract.parse::<contracts::Address>().is_err() {
            return Err(Error::InvalidHostRequest {
                reason: format!("invalid dynamic contract {}", scope.contract),
            });
        }
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

fn ensure_optional_contract_scope(
    patterns: Option<&[String]>,
    dynamic_contracts: &[DynamicContractScope],
    chain: &str,
    target: &str,
) -> std::result::Result<(), ()> {
    if ensure_optional_scope(patterns, target).is_ok() {
        return Ok(());
    }
    if dynamic_contracts.iter().any(|scope| {
        glob_matches(&scope.chain, chain) && scope.contract.eq_ignore_ascii_case(target)
    }) {
        return Ok(());
    }

    Err(())
}
