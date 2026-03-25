// lint-long-file-override allow-max-lines=300
mod wallet_selector;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

use contextful::ResultContextExt;
use contracts::{Address, Client};
use json_store::JsonStore;

#[cfg(test)]
use crate::keystore::decrypt_private_key;
#[cfg(test)]
use crate::signer::KeySigner;
use crate::{
    chains::{BeamChains, ChainEntry, ensure_client_matches_chain_id, find_chain, load_chains},
    config::{BeamConfig, load_config},
    display::ColorMode,
    ens::{ens_name, resolve_ens_address},
    error::{Error, Result},
    keystore::{KeyStore, StoredWallet, find_wallet, is_address_selector, load_keystore},
    known_tokens::KnownToken,
    output::{OutputMode, with_loading},
};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct InvocationOverrides {
    pub chain: Option<String>,
    pub from: Option<String>,
    pub rpc: Option<String>,
}

#[derive(Clone)]
pub struct BeamApp {
    pub chain_store: JsonStore<BeamChains>,
    pub color_mode: ColorMode,
    pub config_store: JsonStore<BeamConfig>,
    pub keystore_store: JsonStore<KeyStore>,
    pub output_mode: OutputMode,
    pub paths: BeamPaths,
    pub overrides: InvocationOverrides,
}

#[derive(Clone, Debug)]
pub struct BeamPaths {
    pub history: PathBuf,
    pub root: PathBuf,
}

#[derive(Clone, Debug)]
pub struct ResolvedChain {
    pub entry: ChainEntry,
    pub rpc_url: String,
}

#[derive(Clone, Debug)]
pub struct ResolvedToken {
    pub address: Address,
    pub decimals: Option<u8>,
    pub label: String,
}

impl BeamApp {
    pub async fn for_root(
        paths: BeamPaths,
        color_mode: ColorMode,
        output_mode: OutputMode,
        overrides: InvocationOverrides,
    ) -> Result<Self> {
        let config_store = load_config(&paths.root).await?;
        let chain_store = load_chains(&paths.root).await?;
        let keystore_store = load_keystore(&paths.root).await?;

        Ok(Self {
            chain_store,
            color_mode,
            config_store,
            keystore_store,
            output_mode,
            paths,
            overrides,
        })
    }

    pub async fn active_chain(&self) -> Result<ResolvedChain> {
        let config = self.config_store.get().await;
        let selection = self
            .overrides
            .chain
            .clone()
            .unwrap_or_else(|| config.default_chain.clone());
        let chains = self.chain_store.get().await;
        let entry = find_chain(&selection, &chains)?;
        let rpc_url = active_rpc_url(&self.overrides, &config, &entry)?;

        Ok(ResolvedChain { entry, rpc_url })
    }

    pub async fn active_chain_client(&self) -> Result<(ResolvedChain, Client)> {
        let chain = self.active_chain().await?;
        let client = with_loading(
            self.output_mode,
            format!("Connecting to {} RPC...", chain.entry.key),
            async { client_for_chain(&chain).await },
        )
        .await?;
        Ok((chain, client))
    }

    pub async fn active_address(&self) -> Result<Address> {
        self.active_optional_address()
            .await?
            .ok_or(Error::NoDefaultWallet)
    }

    pub async fn active_optional_address(&self) -> Result<Option<Address>> {
        let config = self.config_store.get().await;
        let Some(selection) = self
            .overrides
            .from
            .clone()
            .or(config.default_wallet.clone())
        else {
            return Ok(None);
        };

        self.resolve_wallet_or_address(&selection).await.map(Some)
    }

    pub async fn active_wallet(&self) -> Result<StoredWallet> {
        let config = self.config_store.get().await;
        let selector = self
            .overrides
            .from
            .clone()
            .or(config.default_wallet.clone())
            .ok_or(Error::NoDefaultWallet)?;

        self.resolve_wallet(&selector).await
    }

    #[cfg(test)]
    pub async fn active_signer(&self, password: &str) -> Result<KeySigner> {
        let wallet = self.active_wallet().await?;
        let secret_key = decrypt_private_key(&wallet, password)?;
        KeySigner::from_slice(&secret_key)
    }

    pub async fn resolve_wallet(&self, value: &str) -> Result<StoredWallet> {
        let wallets = self.keystore_store.get().await;
        if let Ok(wallet) = find_wallet(&wallets.wallets, value) {
            return Ok(wallet.clone());
        }

        let name = ens_name(value).ok_or_else(|| wallet_not_found(value))?;
        let address = resolve_ens_address(self, &name)
            .await?
            .ok_or_else(|| Error::EnsNameNotFound { name })?;
        let address = format!("{address:#x}");

        wallets
            .wallets
            .iter()
            .find(|wallet| wallet.address.eq_ignore_ascii_case(&address))
            .cloned()
            .ok_or_else(|| wallet_not_found(value))
    }

    pub async fn resolve_wallet_or_address(&self, value: &str) -> Result<Address> {
        if is_address_selector(value) {
            return parse_address(value);
        }

        let wallets = self.keystore_store.get().await;
        if let Ok(wallet) = find_wallet(&wallets.wallets, value) {
            return parse_address(&wallet.address);
        }

        let name = ens_name(value).ok_or_else(|| wallet_not_found(value))?;

        resolve_ens_address(self, &name)
            .await?
            .ok_or(Error::EnsNameNotFound { name })
    }

    pub async fn token_for_chain(&self, input: &str, chain_key: &str) -> Result<ResolvedToken> {
        let config = self.config_store.get().await;

        if let Ok(address) = parse_address(input) {
            let address_label = format!("{address:#x}");
            if let Some((_, known_token)) = config.known_token_by_address(chain_key, &address_label)
            {
                return Ok(ResolvedToken {
                    address,
                    decimals: Some(known_token.decimals),
                    label: known_token.label,
                });
            }

            return Ok(ResolvedToken {
                address,
                decimals: None,
                label: address_label,
            });
        }

        let known_token = config
            .known_token_by_label(chain_key, input)
            .map(|(_, token)| token)
            .ok_or_else(|| Error::UnknownToken {
                chain: chain_key.to_string(),
                token: input.to_string(),
            })?;

        Ok(ResolvedToken {
            address: parse_address(&known_token.address)?,
            decimals: Some(known_token.decimals),
            label: known_token.label,
        })
    }

    pub async fn tracked_tokens_for_chain(&self, chain_key: &str) -> Vec<KnownToken> {
        self.config_store
            .get()
            .await
            .tracked_tokens_for_chain(chain_key)
            .into_iter()
            .map(|(_, token)| token)
            .collect()
    }
}

impl BeamPaths {
    pub fn from_env_or_home() -> Result<Self> {
        if let Ok(path) = std::env::var("BEAM_HOME") {
            return Ok(Self::new(PathBuf::from(path)));
        }

        let home = dirs::home_dir().ok_or(Error::BeamHomeNotFound)?;
        Ok(Self::new(home.join(".beam")))
    }

    pub fn new(root: PathBuf) -> Self {
        Self {
            history: root.join("history.txt"),
            root,
        }
    }
}

pub fn parse_address(value: &str) -> Result<Address> {
    value.parse().map_err(|_| Error::InvalidAddress {
        value: value.to_string(),
    })
}

fn wallet_not_found(selector: &str) -> Error {
    Error::WalletNotFound {
        selector: selector.to_string(),
    }
}

fn active_rpc_url(
    invocation: &InvocationOverrides,
    config: &BeamConfig,
    active_entry: &ChainEntry,
) -> Result<String> {
    if let Some(rpc_url) = invocation.rpc.as_ref() {
        return Ok(rpc_url.clone());
    }

    let rpc_url = config
        .rpc_config_for_chain(active_entry)
        .map(|rpc_config| rpc_config.default_rpc)
        .ok_or_else(|| Error::NoRpcConfigured {
            chain: active_entry.key.clone(),
        })?;

    Ok(rpc_url)
}

async fn client_for_chain(chain: &ResolvedChain) -> Result<Client> {
    let client = Client::try_new(&chain.rpc_url, None).map_err(|_| Error::InvalidRpcUrl {
        value: chain.rpc_url.clone(),
    })?;
    ensure_client_matches_chain_id(&chain.entry.key, chain.entry.chain_id, &client).await?;
    Ok(client)
}

pub fn ensure_root_dir(root: &Path) -> Result<()> {
    std::fs::create_dir_all(root).context("create beam home directory")?;
    #[cfg(unix)]
    {
        std::fs::set_permissions(root, std::fs::Permissions::from_mode(0o700))
            .context("set beam home directory permissions")?;
    }
    Ok(())
}
