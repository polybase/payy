pub mod adapter;
pub mod interface;
pub mod state;

use std::sync::Arc;

use contextful::ResultContextExt;
use contracts::Address;
use payy_evm_client::{BaseClient, PayyNetworkConfig, PrivacyAccount, PrivacyClient};

use crate::{
    chains::ChainEntry,
    commands::signing::prompt_active_private_key,
    error::{Error, Result},
    privacy::adapter::BeamEvmAdapter,
    privacy_config::PrivacyProfile,
    runtime::{BeamApp, ResolvedChain},
};

#[derive(Clone)]
pub struct PrivacyContext {
    pub account: PrivacyAccount,
    pub adapter: Arc<BeamEvmAdapter>,
    pub chain: ResolvedChain,
    pub client: PrivacyClient,
    pub evm_address: Address,
    pub profile: PrivacyProfile,
}

impl PrivacyContext {
    pub fn privacy_address_hex(&self) -> String {
        hex32(&self.account.privacy_address().bytes)
    }
}

pub async fn load_privacy_context(app: &BeamApp, feature: &'static str) -> Result<PrivacyContext> {
    let (chain, evm_client) = app.active_chain_client().await?;
    let profile = privacy_profile(&chain.entry, feature)?;
    let wallet = app.active_wallet().await?;
    let private_key = prompt_active_private_key(app).await?;
    let evm_address = wallet.address.parse().map_err(|_| Error::InvalidAddress {
        value: wallet.address.clone(),
    })?;
    let adapter = Arc::new(BeamEvmAdapter::new(evm_client));
    let network = PayyNetworkConfig {
        chain_id: chain.entry.chain_id,
        privacy_bridge: address_to_bytes(profile.bridge_address()?),
    };
    let client = BaseClient::builder(network, adapter.clone())
        .raw_transaction_submitter(adapter.clone())
        .build()
        .with_evm_private_key(private_key)
        .context("configure beam privacy key")?;
    validate_privacy_readiness(&client, &chain, &profile).await?;
    let account = client
        .privacy()
        .default_account()
        .context("load beam privacy account")?
        .ok_or(Error::NoDefaultWallet)?;

    Ok(PrivacyContext {
        account,
        adapter,
        chain,
        client,
        evm_address,
        profile,
    })
}

pub(crate) async fn validate_privacy_readiness(
    client: &PrivacyClient,
    chain: &ResolvedChain,
    profile: &PrivacyProfile,
) -> Result<()> {
    client
        .privacy()
        .validate_read_chain()
        .await
        .context("validate beam privacy chain")?;
    client
        .privacy()
        .bridge()
        .get_root()
        .await
        .context("read beam privacy bridge root")?;
    let (chain_key_x, chain_key_y) = client
        .privacy()
        .bridge()
        .get_chain_public_key()
        .await
        .context("read beam privacy bridge chain public key")?;
    validate_chain_public_key(
        &chain.entry.key,
        profile.bridge_address()?,
        &chain_key_x,
        &chain_key_y,
    )
}

pub(crate) fn validate_chain_public_key(
    chain: &str,
    bridge: Address,
    chain_key_x: &[u8; 32],
    chain_key_y: &[u8; 32],
) -> Result<()> {
    if is_zero_word(chain_key_x) && is_zero_word(chain_key_y) {
        return Err(Error::PrivacyFeatureUnsupported {
            chain: chain.to_string(),
            feature: format!("bridge chain public key is not configured: {bridge:#x}"),
        });
    }
    Ok(())
}

pub fn privacy_profile(chain: &ChainEntry, feature: &'static str) -> Result<PrivacyProfile> {
    let profile = chain
        .privacy
        .clone()
        .ok_or_else(|| Error::PrivacyNotConfigured {
            chain: chain.key.clone(),
        })?;
    profile.validate()?;
    if !feature_enabled(&profile, feature) {
        return Err(Error::PrivacyFeatureUnsupported {
            chain: chain.key.clone(),
            feature: feature.to_string(),
        });
    }
    Ok(profile)
}

pub fn parse_privacy_address(value: &str) -> Result<payy_evm_client::PrivacyAddress> {
    let raw = value.strip_prefix("0x").unwrap_or(value);
    let bytes = hex::decode(raw).map_err(|_| Error::InvalidPrivateAddress {
        value: value.to_string(),
    })?;
    let bytes = <[u8; 32]>::try_from(bytes).map_err(|_| Error::InvalidPrivateAddress {
        value: value.to_string(),
    })?;
    let address = payy_evm_client::PrivacyAddress::new(bytes);
    address
        .public_key()
        .map_err(|_| Error::InvalidPrivateAddress {
            value: value.to_string(),
        })?;
    Ok(address)
}

pub fn parse_bytes32(value: &str) -> Result<[u8; 32]> {
    let raw = value.strip_prefix("0x").unwrap_or(value);
    let bytes = hex::decode(raw).map_err(|_| Error::InvalidBytes32Value {
        value: value.to_string(),
    })?;
    <[u8; 32]>::try_from(bytes).map_err(|_| Error::InvalidBytes32Value {
        value: value.to_string(),
    })
}

pub fn address_to_bytes(address: Address) -> [u8; 20] {
    address.0
}

pub fn bytes_to_address(bytes: [u8; 20]) -> Address {
    Address::from(bytes)
}

pub fn hex20(bytes: &[u8; 20]) -> String {
    format!("0x{}", hex::encode(bytes))
}

pub fn hex32(bytes: &[u8; 32]) -> String {
    format!("0x{}", hex::encode(bytes))
}

fn feature_enabled(profile: &PrivacyProfile, feature: &str) -> bool {
    match feature {
        "address" | "balance" | "state" => true,
        "mint" => profile.features.mint,
        "burn" => profile.features.burn,
        "direct_send" => profile.features.direct_send,
        "incoming" => profile.features.incoming,
        "claim" => profile.features.claim,
        "claim_links" => profile.features.claim_links,
        "ephemeral" => profile.features.ephemeral,
        "private_payments" => profile.features.private_payments,
        _ => false,
    }
}

fn is_zero_word(value: &[u8; 32]) -> bool {
    value.iter().all(|byte| *byte == 0)
}
