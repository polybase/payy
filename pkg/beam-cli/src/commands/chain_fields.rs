use crate::{
    chains::{BeamChains, chain_key, find_chain},
    error::{Error, Result},
    human_output::sanitize_control_chars_trimmed,
};

const DEFAULT_NATIVE_SYMBOL: &str = "ETH";

pub(super) fn marker(active: bool) -> String {
    if active {
        "*".to_string()
    } else {
        String::new()
    }
}

pub(super) fn normalize_chain_name(name: String) -> Result<String> {
    let name = sanitize_control_chars_trimmed(&name);
    if name.is_empty() {
        return Err(Error::InvalidChainName { name });
    }

    Ok(name)
}

pub(super) fn validate_new_chain_name(name: &str, configured: &BeamChains) -> Result<()> {
    let key = chain_key(name);
    if let Ok(existing_chain) = find_chain(&key, configured) {
        return Err(if existing_chain.key == key {
            Error::ChainNameAlreadyExists {
                name: name.to_string(),
            }
        } else {
            Error::ChainNameConflictsWithSelector {
                name: name.to_string(),
            }
        });
    }

    Ok(())
}

pub(super) fn normalize_native_symbol(native_symbol: Option<String>) -> String {
    native_symbol
        .map(|value| sanitize_control_chars_trimmed(&value).to_ascii_uppercase())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| DEFAULT_NATIVE_SYMBOL.to_string())
}
