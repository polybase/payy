use crate::{
    error::Result,
    privacy::{
        PrivacyContext,
        state::{PrivacyState, PrivacyStateKey},
    },
    runtime::{BeamApp, ResolvedToken},
};

pub(super) fn state_key(ctx: &PrivacyContext) -> PrivacyStateKey {
    PrivacyStateKey {
        bridge: ctx.profile.bridge.clone(),
        chain: ctx.chain.entry.key.clone(),
        chain_id: ctx.chain.entry.chain_id,
        privacy_address: ctx.privacy_address_hex(),
        standard: ctx.profile.standard.clone(),
        standard_version: ctx.profile.version,
        wallet_address: format!("{:#x}", ctx.evm_address),
    }
}

pub(super) fn element_to_u256(value: element::Element) -> contracts::U256 {
    contracts::U256::from_big_endian(&value.to_be_bytes())
}

pub(super) fn u256_to_element(value: contracts::U256) -> element::Element {
    let mut bytes = [0u8; 32];
    value.to_big_endian(&mut bytes);
    element::Element::from_be_bytes(bytes)
}

pub(super) async fn resolve_token(
    app: &BeamApp,
    ctx: &PrivacyContext,
    token: &str,
) -> Result<ResolvedToken> {
    app.token_for_chain(token, &ctx.chain.entry.key).await
}

pub(super) fn save_checkpoint(
    state: &mut PrivacyState,
    key: &PrivacyStateKey,
    token: &ResolvedToken,
    checkpoint: payy_evm_client::OwnedNoteState,
) -> Result<()> {
    state
        .entry_mut(key)?
        .token_mut(&format!("{:#x}", token.address))
        .checkpoint = Some(checkpoint);
    Ok(())
}

pub(super) fn field_to_address(field: element::Element) -> [u8; 20] {
    let bytes = field.to_be_bytes();
    let mut out = [0u8; 20];
    out.copy_from_slice(&bytes[12..]);
    out
}
