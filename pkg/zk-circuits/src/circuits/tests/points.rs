use crate::circuits::generated::points::PointsInput;
use crate::circuits::generated::submodules::common::Note as CommonNote;
use crate::{Prove, Result, Verify};
use barretenberg_cli::CliBackend;
use element::Element;
use zk_primitives::{bridged_polygon_usdc_note_kind, get_address_for_private_key};

#[tokio::test]
async fn test_points_prove_and_verify() -> Result<()> {
    // Match the Noir `points` test pattern with one padding note and nine owned notes.
    let secret_key = Element::new(101);
    let address = get_address_for_private_key(secret_key);

    let primitive_notes = std::array::from_fn(|i| {
        if i == 0 {
            zk_primitives::Note::padding_note()
        } else {
            zk_primitives::Note::new_with_psi(
                address,
                Element::new((i * 10) as u64),
                Element::new((i + 1) as u64),
                bridged_polygon_usdc_note_kind(),
            )
        }
    });

    let secret_keys = std::array::from_fn(|i| if i == 0 { Element::ZERO } else { secret_key });
    let commitments = primitive_notes
        .each_ref()
        .map(zk_primitives::Note::commitment);

    let timestamp = Element::new(1234567890u64);
    let value = primitive_notes
        .each_ref()
        .map(|note| note.value)
        .into_iter()
        .sum();
    let hash = hash::hash_merge([timestamp, address]);
    let notes = primitive_notes.map(CommonNote::from);

    let input = PointsInput {
        notes,
        secret_keys,
        address,
        timestamp,
        value,
        hash,
        commitments,
    };

    let backend = CliBackend;
    let proof = input.prove(&backend).await?;
    assert!(
        proof.verify(&backend).await.is_ok(),
        "Proof verification failed"
    );

    Ok(())
}
