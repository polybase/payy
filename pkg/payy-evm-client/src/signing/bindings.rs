use contextful::ResultContextExt;
use payy_evm_client_interface::{B256, Result};

pub(crate) fn schnorr_construct_signature(
    message: B256,
    private_key: B256,
) -> Result<(B256, B256)> {
    Ok(
        barretenberg_rs::schnorr_construct_signature(&message, private_key)
            .context("construct schnorr signature with bb bindings")?,
    )
}
