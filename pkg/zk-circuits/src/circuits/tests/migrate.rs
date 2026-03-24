use crate::circuits::generated::migrate::MigrateInput;
use crate::{Prove, Result, Verify};
use barretenberg_cli::CliBackend;
use element::Element;

#[tokio::test]
async fn test_migrate_proof_generation_and_verification() -> Result<()> {
    let owner_pk = Element::from(101u64);
    let old_address = hash_poseidon::hash_merge([owner_pk, Element::ZERO]);
    let new_address = hash::hash_merge([owner_pk, Element::ZERO]);
    let input = MigrateInput {
        owner_pk,
        old_address,
        new_address,
    };
    let backend = CliBackend;

    let proof = input.prove(&backend).await?;
    proof.verify(&backend).await?;

    Ok(())
}
