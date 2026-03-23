use async_trait::async_trait;
use barretenberg_interface::{BbBackend, Error, Result};

const DISABLED_MESSAGE: &str = "bb_rs feature not enabled; run the mobile rustbridge build script";

pub struct BindingBackend;

fn bindings_disabled<T>() -> Result<T> {
    Err(Error::Backend(DISABLED_MESSAGE.to_owned()))
}

#[async_trait]
impl BbBackend for BindingBackend {
    async fn prove(
        &self,
        _program: &[u8],
        _bytecode: &[u8],
        _key: &[u8],
        _witness: &[u8],
        _oracle_hash_keccak: bool,
    ) -> Result<Vec<u8>> {
        bindings_disabled()
    }

    async fn verify(
        &self,
        _proof: &[u8],
        _public_inputs: &[u8],
        _key: &[u8],
        _oracle_hash_keccak: bool,
    ) -> Result<()> {
        bindings_disabled()
    }
}
