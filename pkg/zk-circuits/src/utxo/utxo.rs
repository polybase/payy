use std::array;

use crate::{
    chips::{
        add::AddCulmChip, aggregation::snark::Snark,
        binary_decomposition::BinaryDecompositionConfig, is_constant::IsConstantChip,
        poseidon::PoseidonConfig, swap::CondSwapChip,
    },
    constants::{UTXO_INPUTS, UTXO_OUTPUTS},
    data::{InputNote, Note, ParameterSet, Utxo, UtxoKind},
    params::load_params,
    proof::Proof,
    util::{assign_constant, assign_private_input, keygen_from_params},
    CircuitKind,
};
use halo2_base::halo2_proofs::{
    circuit::{Layouter, Value},
    halo2curves::bn256::{Bn256, Fr, G1Affine},
    plonk::{Advice, Column, Error, Instance, ProvingKey, VerifyingKey},
    poly::kzg::commitment::ParamsKZG,
};
use rand::RngCore;
use zk_primitives::Element;

impl UtxoKind {
    pub(crate) fn as_element(&self) -> Fr {
        match self {
            UtxoKind::Null => Fr::from(0u64),
            UtxoKind::Transfer => Fr::from(1u64),
            UtxoKind::Mint => Fr::from(2u64),
            UtxoKind::Burn => Fr::from(3u64),
        }
    }
}

impl<const MERKLE_D: usize> Utxo<MERKLE_D> {
    pub fn new(
        inputs: [InputNote<MERKLE_D>; UTXO_INPUTS],
        outputs: [Note; UTXO_OUTPUTS],
        root: Element,
        kind: UtxoKind,
    ) -> Self {
        Utxo {
            inputs,
            outputs,
            root,
            kind,
        }
    }

    pub fn new_transfer(
        inputs: [InputNote<MERKLE_D>; UTXO_INPUTS],
        outputs: [Note; UTXO_OUTPUTS],
        root: Element,
    ) -> Self {
        Utxo::new(inputs, outputs, root, UtxoKind::Transfer)
    }

    // TODO: do we need root here? Surely its just a padding element
    pub fn new_mint(output_note: Note) -> Self {
        let inputs = array::from_fn(|_| InputNote::padding_note());
        let outputs = [output_note, Note::padding_note()];
        Utxo::new(inputs, outputs, Element::ZERO, UtxoKind::Mint)
    }

    pub fn new_burn(input_note: InputNote<MERKLE_D>, root: Element) -> Self {
        let inputs = [input_note, InputNote::padding_note()];
        let outputs = array::from_fn(|_| Note::padding_note());
        Utxo::new(inputs, outputs, root, UtxoKind::Burn)
    }

    pub fn new_padding() -> Self {
        let inputs: [InputNote<MERKLE_D>; 2] = array::from_fn(|_| InputNote::padding_note());
        let outputs = array::from_fn(|_| Note::padding_note());
        Utxo::new(inputs, outputs, Element::ZERO, UtxoKind::Transfer)
    }

    #[allow(clippy::too_many_arguments)]
    pub fn enforce_constraints(
        &self,
        mut layouter: impl Layouter<Fr>,
        instance: Column<Instance>,
        advice: Column<Advice>,
        poseidon_config: PoseidonConfig<Fr, 3, 2>,
        add_chip: AddCulmChip<Fr>,
        swap_chip: CondSwapChip<Fr>,
        padding_constant_chip: IsConstantChip<Fr>,
        is_mint_chip: IsConstantChip<Fr>,
        is_burn_chip: IsConstantChip<Fr>,
        decompose: BinaryDecompositionConfig<Fr, 1>,
    ) -> Result<(), Error> {
        // Total value
        let mut in_value = vec![];
        let mut out_value = vec![];

        // A set of hashes that we need to verify against public inputs
        let mut input_hashes = vec![];
        let mut output_hashes = vec![];

        // Root of the merkle tree for each input, should all be the same root
        let mut roots = vec![];

        // Witness the root of the merkle tree (in case we need to ignore the merkle tree check)
        let unverified_root = assign_private_input(
            || "unverified root witness",
            layouter.namespace(|| "unverified root witness"),
            advice,
            Value::known(self.root()),
        )?;

        let zero = assign_constant(
            || "unverified padding witness",
            layouter.namespace(|| "unverified root witness"),
            advice,
            Fr::zero(),
        )?;

        // Witness the kind of utxo txn
        let utxo_kind = assign_private_input(
            || "utxo kind witness",
            layouter.namespace(|| "utxo kind witness"),
            advice,
            Value::known(self.kind.as_element()),
        )?;

        // Is mint
        let is_mint = is_mint_chip.assign(layouter.namespace(|| "is mint"), utxo_kind.clone())?;

        // Is burn
        let is_burn = is_burn_chip.assign(layouter.namespace(|| "is burn"), utxo_kind)?;

        for input_note in &self.inputs {
            let cells = input_note.enforce_constraints(
                layouter.namespace(|| "input note"),
                advice,
                poseidon_config.clone(),
                swap_chip.clone(),
                padding_constant_chip.clone(),
            )?;

            // Swap the merkle tree root if the note is padding, we're essentially using Swap
            // as a ternary operator here, so the merkle tree root will be the same if the note
            // is not padding, and if it is padding, the merkle tree root will be the unverified
            // in this case we don't care as this is just padding record with 0 value
            let (root, _) = swap_chip.swap_assigned(
                layouter.namespace(|| "swap padded root?"),
                (&cells.root, &unverified_root),
                &cells.commitment.is_padding,
            )?;

            // Change the nullifier to the default padding nullifier if input note is padding, we
            // want to use the same padding commitment value for easier detection upstream of padding
            // notes
            let (nullifier, _) = swap_chip.swap_assigned(
                layouter.namespace(|| "swap padded root?"),
                (&cells.nullifier, &zero),
                &cells.commitment.is_padding,
            )?;

            roots.push(root);
            input_hashes.push(nullifier);
            in_value.push(cells.commitment.value);
        }

        for output_note in &self.outputs {
            let cells = output_note.enforce_constraints(
                layouter.namespace(|| "output_note"),
                advice,
                poseidon_config.clone(),
                padding_constant_chip.clone(),
                swap_chip.clone(),
            )?;
            let value = cells.value;

            output_hashes.push(cells.cm);
            out_value.push(value.clone());

            // Verify that out_value is MAX 2^240
            // Binary decomposition using RunningSum is a vec of AssignedCells containing the bits
            let decomposed_bits = layouter.assign_region(
                || "decompose",
                |mut region| {
                    // We use non-struct because the merkle tree is not as big as the hash (i.e. we're only
                    // interested in the last n bits)
                    decompose.copy_decompose(&mut region, 0, value.clone(), 256, 256)
                },
            )?;

            layouter.assign_region(
                || "2^240 value range check",
                |mut region| {
                    // Constrain the value to be less than 2^240
                    for bit in decomposed_bits.iter().rev().take(256 - 240) {
                        region.constrain_constant(bit.cell(), Fr::zero())?;
                    }
                    Ok(())
                },
            )?;
        }

        let hashes = input_hashes
            .iter()
            .chain(output_hashes.iter())
            .collect::<Vec<_>>();

        let total_in = add_chip.assign(layouter.namespace(|| "total in"), in_value.as_slice())?;

        let total_out =
            add_chip.assign(layouter.namespace(|| "total out"), out_value.as_slice())?;

        // Set mint/burn hash if minting
        let (mb_hash, _) = swap_chip.swap_assigned(
            layouter.namespace(|| "swap hash to mint hash?"),
            (&zero, &output_hashes[0]),
            &is_mint,
        )?;

        // Set mint/burn hash if burning
        let (mb_hash, _) = swap_chip.swap_assigned(
            layouter.namespace(|| "swap value to burn value?"),
            (&mb_hash, &input_hashes[0]),
            &is_burn,
        )?;

        // Set mint/burn value if minting
        let (value, _) = swap_chip.swap_assigned(
            layouter.namespace(|| "swap value to mint value?"),
            (&zero, &total_out),
            &is_mint,
        )?;

        // Set mint/burn value if burning
        let (value, _) = swap_chip.swap_assigned(
            layouter.namespace(|| "swap value to burn value?"),
            (&value, &total_in),
            &is_burn,
        )?;

        // Swap total_out if minting
        let (total_out, _) = swap_chip.swap_assigned(
            layouter.namespace(|| "swap total_in to mint value?"),
            (&total_out, &zero),
            &is_mint,
        )?;

        // Swap total_in if burning
        let (total_in, _) = swap_chip.swap_assigned(
            layouter.namespace(|| "swap total_in to burn value?"),
            (&total_in, &zero),
            &is_burn,
        )?;

        // Check value total_in == total_out!
        layouter.assign_region(
            || "constrain total_in == total_out",
            |mut region| region.constrain_equal(total_in.cell(), total_out.cell()),
        )?;

        // Check roots are valid
        for hash in roots.iter() {
            layouter.constrain_instance(hash.cell(), instance, 0)?;
        }

        // Constrain kind to public input, so we know what rules have been applied
        // TODO: do we need this, can we just check if value has an output, meaning it is a mint/burn
        layouter.constrain_instance(mb_hash.cell(), instance, 1)?;

        // Constrain value to public input (value will be non-zero if minting or burning)
        layouter.constrain_instance(value.cell(), instance, 2)?;

        // Verify hashes aginst inputs
        for (i, hash) in hashes.iter().enumerate() {
            layouter.constrain_instance(hash.cell(), instance, i + 3)?;
        }

        Ok(())
    }

    /// Public inputs to be used in proof, public inputs need to have a determinsitc ordering
    /// so we can constrain them correctly - ordering is:
    ///  - input.merkle_root x inputs
    ///  - input.nullifier x inputs
    ///  - output.commitment x outputs
    pub fn public_inputs(&self) -> Vec<Fr> {
        let mut hashes = vec![];

        // Push the root of the merkle tree as a witness
        hashes.push(self.root());

        // Push the input/output hash, used for mint/burn only
        hashes.push(match self.kind {
            UtxoKind::Mint => self.outputs[0].commitment().into(),
            UtxoKind::Burn => self.inputs[0].nullifer().into(),
            _ => Fr::zero(),
        });

        // Output value (only when minting/burning)
        hashes.push(match self.kind {
            UtxoKind::Mint => self.outputs[0].value().into(),
            UtxoKind::Burn => self.inputs[0].value().into(),
            _ => Fr::zero(),
        });

        // input notes use the same merkle root
        for input_note in &self.inputs {
            hashes.push(input_note.nullifer().into())
        }

        for output_note in &self.outputs {
            hashes.push(output_note.commitment().into())
        }

        hashes
    }

    pub fn root(&self) -> Fr {
        self.root.into()
    }

    pub fn leafs(&self) -> Vec<Fr> {
        let mut hashes = vec![];

        for input_note in &self.inputs {
            hashes.push(input_note.nullifer().into())
        }

        for output_note in &self.outputs {
            hashes.push(output_note.commitment().into())
        }

        hashes
    }

    pub fn prove(
        &self,
        params: &ParamsKZG<Bn256>,
        pk: &ProvingKey<G1Affine>,
        rng: impl RngCore,
    ) -> Result<Proof, Error> {
        let circuit = Self::default();
        let instance = self.public_inputs();
        let instances = &[instance.as_slice()];
        Proof::create(params, pk, circuit, instances, rng)
    }

    pub fn snark(&self, kind: CircuitKind) -> Result<Snark, crate::Error> {
        let (pk, _) = self.keygen(kind.params());

        Snark::create(
            self.clone(),
            vec![self.public_inputs()],
            load_params(kind.params()),
            &pk,
        )
        .map_err(crate::Error::err)
    }

    pub fn keygen(&self, params: ParameterSet) -> (ProvingKey<G1Affine>, VerifyingKey<G1Affine>) {
        keygen_from_params(params, self)
    }
}
