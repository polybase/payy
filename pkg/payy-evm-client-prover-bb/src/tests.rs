use std::sync::Arc;

use async_trait::async_trait;
use barretenberg_interface::{BbBackend, Result as BbResult};
use payy_evm_client_prover_interface::{
    Error, PrivacyCircuit, PrivacyProver, ProofDecodeErrorKind, ProveRequest,
};

use crate::BarretenbergPrivacyProver;

#[derive(Debug)]
struct MalformedProofBackend;

#[async_trait]
impl BbBackend for MalformedProofBackend {
    async fn prove(
        &self,
        _program: &[u8],
        _bytecode: &[u8],
        _key: &[u8],
        _witness: &[u8],
        _oracle: bool,
    ) -> BbResult<Vec<u8>> {
        Ok(vec![0u8; 31])
    }

    async fn verify(
        &self,
        _proof: &[u8],
        _public_inputs: &[u8],
        _key: &[u8],
        _oracle: bool,
    ) -> BbResult<()> {
        Ok(())
    }
}

#[tokio::test]
async fn malformed_backend_proof_returns_typed_error() {
    let prover = BarretenbergPrivacyProver::new(Arc::new(MalformedProofBackend));
    let err = prover
        .prove(ProveRequest {
            circuit: PrivacyCircuit::Mint,
            witness: Vec::new(),
        })
        .await
        .unwrap_err();

    assert!(matches!(
        err,
        Error::InvalidProof {
            circuit: PrivacyCircuit::Mint,
            kind: ProofDecodeErrorKind::PublicInputsTooShort,
        }
    ));
}
