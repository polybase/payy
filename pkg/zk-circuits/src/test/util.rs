use halo2_base::halo2_proofs::{
    halo2curves::{
        bn256::{Bn256, Fr, G1Affine},
        FieldExt,
    },
    plonk::{
        keygen_pk, keygen_vk, Advice, Circuit, Column, ConstraintSystem, Error, Instance,
        ProvingKey, VerifyingKey,
    },
    poly::kzg::commitment::ParamsKZG,
};
use itertools::Itertools;
use smirk::Element;

use crate::{
    chips::{
        aggregation::snark::Snark,
        is_constant::{IsConstantChip, IsConstantConfig},
        poseidon::{P128Pow5T3Fr, PoseidonChip, PoseidonConfig},
        swap::{CondSwapChip, CondSwapConfig},
    },
    data::MerklePath,
};

pub fn poseidon_config(meta: &mut ConstraintSystem<Fr>) -> PoseidonConfig<Fr, 3, 2> {
    let advices = [
        meta.advice_column(),
        meta.advice_column(),
        meta.advice_column(),
        meta.advice_column(),
        meta.advice_column(),
    ];

    for advice in advices.iter() {
        meta.enable_equality(*advice);
    }

    let lagrange_coeffs = [
        meta.fixed_column(),
        meta.fixed_column(),
        meta.fixed_column(),
        meta.fixed_column(),
        meta.fixed_column(),
        meta.fixed_column(),
        meta.fixed_column(),
    ];
    meta.enable_constant(lagrange_coeffs[0]);

    PoseidonChip::configure::<P128Pow5T3Fr>(
        meta,
        advices[1..4].try_into().unwrap(),
        advices[0],
        lagrange_coeffs[1..4].try_into().unwrap(),
        lagrange_coeffs[4..7].try_into().unwrap(),
    )
}

pub fn is_padding_config<F: FieldExt>(
    meta: &mut ConstraintSystem<F>,
    constant_val: F,
) -> IsConstantConfig<F> {
    let a = advice_column_equality(meta);
    let b = advice_column_equality(meta);
    let c = advice_column_equality(meta);
    IsConstantChip::configure(meta, a, b, c, constant_val)
}

pub fn swap_config<F: FieldExt>(meta: &mut ConstraintSystem<F>) -> CondSwapConfig {
    let advices = [
        advice_column_equality(meta),
        advice_column_equality(meta),
        advice_column_equality(meta),
        advice_column_equality(meta),
        advice_column_equality(meta),
    ];
    CondSwapChip::configure(meta, advices[0..5].try_into().unwrap())
}

pub fn advice_column_equality<F: FieldExt>(meta: &mut ConstraintSystem<F>) -> Column<Advice> {
    let advice = meta.advice_column();
    meta.enable_equality(advice);
    advice
}

pub fn instance_column_equality<F: FieldExt>(meta: &mut ConstraintSystem<F>) -> Column<Instance> {
    let instance = meta.instance_column();
    meta.enable_equality(instance);
    instance
}

pub fn get_params<C: Circuit<Fr>>(
    k: u32,
    circuit: &C,
) -> (
    ParamsKZG<Bn256>,
    VerifyingKey<G1Affine>,
    ProvingKey<G1Affine>,
) {
    let params = halo2_base::utils::fs::gen_srs(k);
    let vk = keygen_vk(&params, circuit).expect("keygen_vk should not fail");
    let pk = keygen_pk(&params, vk.clone(), circuit).expect("keygen_pk should not fail");

    (params, vk, pk)
}

pub fn get_snark<C: Circuit<Fr>>(k: u32, circuit: C, instances: Vec<Fr>) -> Result<Snark, Error> {
    let (params, _, pk) = get_params(k, &circuit);
    Snark::create(circuit, vec![instances], &params, &pk)
}

pub fn apply_two_merkle_leaves<const DEPTH: usize>(
    leaf_1: Element,
    leaf_2: Element,
) -> MerklePath<DEPTH> {
    let default_path = MerklePath::<DEPTH>::default();
    let applied_path = default_path.apply_leaf(leaf_1);

    // Calculate the computed leaves based on leaf_1 on default tree, from high to low
    let computed_siblings = applied_path.siblings.iter().rev().collect_vec();

    let leaf_1_bits = leaf_1.lsb(DEPTH - 1).into_iter().collect_vec();

    let leaf_2_bits = leaf_2.lsb(DEPTH - 1).into_iter().collect_vec();

    // let leaf_1_bits = MerklePath::<DEPTH>::bits(&leaf_1);
    // let leaf_1_bits = leaf_1_bits.iter().map(|b| *b).rev().collect_vec();
    // let leaf_2_bits = MerklePath::<N>::bits(&leaf_2);
    // let leaf_2_bits = leaf_2_bits.iter().map(|b| *b).rev().collect_vec();

    let mut second_path_siblings = MerklePath::<DEPTH>::default()
        .siblings
        .into_iter()
        .rev()
        .collect_vec();

    for (i, _) in computed_siblings.iter().enumerate() {
        if leaf_1_bits[i] != leaf_2_bits[i] {
            second_path_siblings[i] = *computed_siblings[i];
            break;
        }
    }

    second_path_siblings.reverse();

    let new_sibs_path = MerklePath {
        siblings: second_path_siblings,
    };

    assert_eq!(
        default_path.compute_root(leaf_1),
        new_sibs_path.compute_null_root(leaf_2),
        "leaf_1 new root, must match leaf_2 old root"
    );

    new_sibs_path
}

#[cfg(test)]
mod tests {
    use crate::util::insecure_random_element;

    use super::*;

    #[test]
    fn test_apply_two_merkle_leaves() {
        let leaf_1 = Element::from(4u64);
        let leaf_2 = Element::from(6u64);

        apply_two_merkle_leaves::<3>(leaf_1, leaf_2);
    }

    #[test]
    fn test_apply_two_merkle_leaves_0_6() {
        let leaf_1 = Element::from(1u64);
        let leaf_2 = Element::from(6u64);

        apply_two_merkle_leaves::<3>(leaf_1, leaf_2);
    }

    #[test]
    fn test_apply_two_merkle_leaves_big_tree() {
        let leaf_1 = insecure_random_element();
        let leaf_2 = insecure_random_element();

        apply_two_merkle_leaves::<32>(leaf_1, leaf_2);
    }
}
