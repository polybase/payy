use crate::circuits::generated::signature::SignatureInput;
use crate::{Prove, Result, Verify};
use barretenberg_cli::CliBackend;
use element::Element;

#[tokio::test]
async fn test_signature_proof_generation_and_verification() -> Result<()> {
    let owner_pk = Element::from(101u64);
    let message = Element::from(100u64);
    let input = SignatureInput {
        owner_pk,
        message_hash: hash::hash_merge([owner_pk, message]),
        address: hash::hash_merge([owner_pk, Element::ZERO]),
        message,
    };
    let backend = CliBackend;

    let proof = input.prove(&backend).await?;
    proof.verify(&backend).await?;

    Ok(())
}
