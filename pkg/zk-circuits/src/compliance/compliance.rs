use crate::{
    chips::{
        is_constant::IsConstantChip, merkle_path::MerklePathInclusionConstrainCells,
        poseidon::PoseidonConfig, swap::CondSwapChip,
    },
    data::{MerklePath, Note, ParameterSet},
    util::{assign_constant, keygen_from_params},
};
use halo2_base::halo2_proofs::{
    circuit::Layouter,
    halo2curves::bn256::{Fr, G1Affine},
    plonk::{Advice, Column, Error, Instance, ProvingKey, VerifyingKey},
};

/// Compliance proves that the source of a note was not from a known bad actor
#[derive(Clone, Debug, Default)]
pub struct Compliance<const N: usize> {
    /// Note that we want to prove compliance for
    note: Note,

    /// Expected recent root of the compliance merkle tree
    #[allow(unused)]
    recent_root: Fr,

    /// Merkle tree path for compliance merkle tree, so we can prove that the source does not exist in the
    /// merkle tree
    merkle_path: MerklePath<N>,
}

impl<const N: usize> Compliance<N> {
    pub fn new(note: Note, recent_root: Fr, merkle_path: MerklePath<N>) -> Self {
        Self {
            note,
            recent_root,
            merkle_path,
        }
    }

    /// Enforces constraints for the input note (includes default note constraints, plus additional
    /// constraints to prove spending of note is allowable)
    #[allow(clippy::too_many_arguments)]
    pub fn enforce_constraints(
        &self,
        mut layouter: impl Layouter<Fr>,
        advice: Column<Advice>,
        instance: Column<Instance>,
        poseidon_config: PoseidonConfig<Fr, 3, 2>,
        swap_chip: CondSwapChip<Fr>,
        is_zero_chip: IsConstantChip<Fr>,
    ) -> Result<(), Error> {
        // First we need to check the std note constraints
        // TODO(sec): update note to include SOURCE
        let note_commitment_cells = self.note.enforce_constraints(
            layouter.namespace(|| "input note enforce commitment"),
            advice,
            poseidon_config.clone(),
            is_zero_chip,
            swap_chip.clone(),
        )?;

        // Witness null leaf
        let null_leaf = assign_constant(
            || "null leaf witness",
            layouter.namespace(|| "null leaf witness"),
            advice,
            Fr::zero(),
        )?;

        // Check input note commitment is in an existing merkle root
        let MerklePathInclusionConstrainCells { root } =
            self.merkle_path.enforce_inclusion_constraints(
                layouter.namespace(|| "leaf in tree"),
                // TODO(sec): this should come from note_commitment_cells
                self.note.source().into(),
                null_leaf,
                poseidon_config,
                swap_chip,
            )?;

        // Constrain calculated root from null merkle path to be equal to the recent root
        // provided. Recent root must be checked against the compliance merkle tre.
        layouter.constrain_instance(root.cell(), instance, 0)?;

        // Constrain the note commitment, so we know which note to allow
        layouter.constrain_instance(note_commitment_cells.cm.cell(), instance, 1)?;

        Ok(())
    }

    pub fn keygen(&self, params: ParameterSet) -> (ProvingKey<G1Affine>, VerifyingKey<G1Affine>) {
        keygen_from_params(params, self)
    }
}
