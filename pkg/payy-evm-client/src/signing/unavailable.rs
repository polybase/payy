use payy_evm_client_interface::{B256, Error, Result};

pub(crate) fn schnorr_construct_signature(
    _message: B256,
    _private_key: B256,
) -> Result<(B256, B256)> {
    Err(Error::MissingCapability {
        capability: "bb_backend",
    })
}
