use crate::error::Result;

#[unimock::unimock(api = BbBackendMock)]
#[async_trait::async_trait]
pub trait BbBackend: Send + Sync {
    async fn prove(
        &self,
        program: &[u8],
        bytecode: &[u8],
        key: &[u8],
        witness: &[u8],
        oracle: bool,
    ) -> Result<Vec<u8>>;
    async fn verify(
        &self,
        proof: &[u8],
        public_inputs: &[u8],
        key: &[u8],
        oracle: bool,
    ) -> Result<()>;
}

impl std::fmt::Debug for dyn BbBackend {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("BbBackend")
    }
}
