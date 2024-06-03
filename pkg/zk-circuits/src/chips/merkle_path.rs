use crate::{
    chips::{
        poseidon::{poseidon_hash, poseidon_hash_gadget, PoseidonConfig},
        swap::CondSwapChip,
    },
    data::MerklePath,
};
use halo2_base::halo2_proofs::{
    circuit::{AssignedCell, Layouter, Value},
    halo2curves::bn256::Fr,
    plonk::Error,
};
use smirk::Element;
use std::iter::zip;

impl<const DEPTH: usize> MerklePath<DEPTH> {
    pub fn new(siblings: Vec<Element>) -> Self {
        assert_eq!(DEPTH - 1, siblings.len(), "Merkle path invalid size");

        MerklePath { siblings }
    }

    // Calcualte a MerklePath based on the current leaf
    pub fn apply_leaf(&self, leaf: Element) -> MerklePath<DEPTH> {
        MerklePath {
            siblings: self.compute_path_from_leaf(leaf)[0..DEPTH - 1].to_vec(),
        }
    }

    /// Compute the root hash of a tree with the given hash at this path
    pub fn compute_root(&self, hash: Element) -> Element {
        self.compute_root_from_leaf(&hash, hash)
    }

    pub fn compute_null_root(&self, hash: Element) -> Element {
        self.compute_root_from_leaf(&hash, Element::ZERO)
    }

    /// Compute the root hash of a tree with the given hash at this path
    fn compute_root_from_leaf(&self, hash_path: &Element, leaf: Element) -> Element {
        let bits = Self::least_significant_bits(*hash_path);

        let mut hash = leaf.to_base();

        for (is_right, &sibling) in zip(bits, &self.siblings) {
            match is_right {
                true => hash = poseidon_hash([sibling.to_base(), hash]),
                false => hash = poseidon_hash([hash, sibling.to_base()]),
            }
        }

        hash.into()
    }

    pub fn compute_path_from_leaf(&self, leaf: Element) -> Vec<Element> {
        let bits = Self::least_significant_bits(leaf);

        let mut path = vec![leaf];
        let mut hash = leaf.to_base();

        for (is_right, &sibling) in zip(bits, &self.siblings) {
            // TODO: make Element hashable
            match is_right {
                true => hash = poseidon_hash([sibling.to_base(), hash]),
                false => hash = poseidon_hash([hash, sibling.to_base()]),
            };
            path.push(hash.into())
        }

        path
    }

    /// `bits` takes the first N least significant bits from the field element and returns the bits in
    /// reverse order.
    ///
    /// For example assuming N is 2 (max 2 bis):
    ///
    /// Int |  Bit     |   Returns
    /// ----|----------|----------------
    /// `2` |  `010`   |  `[0, 1]`
    /// `3` |  `011`   |  `[1, 1]`
    /// `4` |  `100`   |  `[0, 0]`
    /// `5` |  `101`   |  `[1, 0]`
    ///
    // pub fn bits(hash_path: &Fr) -> BitVec<u64> {
    //     let bits: BitArray<[u64; 4]> = hash_path.to_le_bits();
    //     let bits = Self::last_n_bits(&bits);
    //     BitVec::from_bitslice(bits)
    // }
    //
    // fn last_n_bits(bits: &BitArray<[u64; 4]>) -> &BitSlice<u64> {
    //     let slice = &bits[0..N];
    //     assert_eq!(slice.len(), N);
    //     slice
    // }

    pub fn least_significant_bits(element: Element) -> impl Iterator<Item = bool> {
        element.lsb(DEPTH - 1).into_iter().rev()
    }

    pub fn enforce_inclusion_constraints(
        &self,
        mut layouter: impl Layouter<Fr>,
        leaf_value: Fr,
        leaf_assigned: AssignedCell<Fr, Fr>,
        poseidon_config: PoseidonConfig<Fr, 3, 2>,
        swap_chip: CondSwapChip<Fr>,
    ) -> Result<MerklePathInclusionConstrainCells, Error> {
        let decomposed_bits =
            MerklePath::<DEPTH>::least_significant_bits(Element::from(leaf_value));

        let siblings = self
            .siblings
            .iter()
            .map(|e| e.to_base())
            .map(Value::known)
            .zip(decomposed_bits.map(|b| Value::known(if b { Fr::one() } else { Fr::zero() })))
            .collect::<Vec<_>>();

        let root = merkle_root_value(
            layouter.namespace(|| "new root"),
            swap_chip,
            poseidon_config,
            leaf_assigned,
            siblings.as_slice(),
        )?;

        Ok(MerklePathInclusionConstrainCells { root })
    }
}

pub struct MerklePathInclusionConstrainCells {
    pub root: AssignedCell<Fr, Fr>,
}

// TODO: refactor these!

/// Get the merkle root based on leaf + (siblings + LR directions)
#[allow(clippy::type_complexity)]
pub fn merkle_root(
    mut layouter: impl Layouter<Fr>,
    swap_chip: CondSwapChip<Fr>,
    poseidon_config: PoseidonConfig<Fr, 3, 2>,
    leaf: AssignedCell<Fr, Fr>,
    // Turple is (node, LR)
    siblings: &[(&AssignedCell<Fr, Fr>, &AssignedCell<Fr, Fr>)],
) -> Result<AssignedCell<Fr, Fr>, Error> {
    let mut cur = leaf;

    for (sibling, swap) in siblings.iter() {
        // Pair, in the correct order (left=0, right=1)
        // TODO: is this the correct way around?!
        let pair = swap_chip.swap_assigned(
            layouter.namespace(|| "merkle path swap"),
            (&cur, sibling),
            swap,
        )?;

        cur = poseidon_hash_gadget(
            poseidon_config.clone(),
            layouter.namespace(|| "merkle poseidon hash"),
            [pair.0, pair.1],
        )?;
    }

    Ok(cur)
}

/// Get the merkle root based on leaf + (siblings + LR directions)
#[allow(clippy::type_complexity)]
pub fn merkle_root_value(
    mut layouter: impl Layouter<Fr>,
    swap_chip: CondSwapChip<Fr>,
    poseidon_config: PoseidonConfig<Fr, 3, 2>,
    leaf: AssignedCell<Fr, Fr>,
    // Turple is (node, LR)
    siblings: &[(Value<Fr>, Value<Fr>)],
) -> Result<AssignedCell<Fr, Fr>, Error> {
    let mut cur = leaf;

    for (sibling, swap) in siblings.iter() {
        // Pair, in the correct order (left=0, right=1)
        // TODO: is this the correct way around?!
        let pair = swap_chip.swap(
            layouter.namespace(|| "merkle path swap"),
            (&cur, *sibling),
            *swap,
        )?;

        cur = poseidon_hash_gadget(
            poseidon_config.clone(),
            layouter.namespace(|| "merkle poseidon hash"),
            [pair.0, pair.1],
        )?;
    }

    Ok(cur)
}

#[cfg(test)]
mod tests {
    use bitvec::prelude::*;
    use itertools::Itertools;

    use super::*;

    fn hmerge(a: Element, b: Element) -> Element {
        poseidon_hash([a.to_base(), b.to_base()]).into()
    }

    #[test]
    fn first_insert() {
        let empty_tree = MerklePath::<64>::default();

        let root = empty_tree.compute_root(Element::from(3u64)).to_base();

        assert_eq!(
            format!("{root:?}"),
            "0x26debce8a5ba1d092589121944bfc2cc55d858bcd7a697ec2fd1b832b4b20c40"
        );
    }

    #[test]
    fn last_n_bits_no_cutoff_test() {
        // Binary: [1, 0]
        let hash_last_bits = 2u64;
        let hash = Fr::from(hash_last_bits);
        let bits = MerklePath::<3>::least_significant_bits(Element::from(hash));

        // let slice =
        let mut bv: BitVec<u8, Lsb0> = BitVec::new();

        // [0, 1]
        bv.push(false);
        bv.push(true);

        assert_eq!(bits.collect_vec(), bv.into_iter().collect_vec());
    }

    #[test]
    fn last_n_bits_cutoff_test() {
        // Binary: [1, 0, 1]
        let hash_last_bits = 5u64;
        let hash = Fr::from(hash_last_bits);
        let bits = MerklePath::<3>::least_significant_bits(Element::from(hash));

        // let slice =
        let mut bv: BitVec<u8, Lsb0> = BitVec::new();

        // [1, 0]
        bv.push(true);
        bv.push(false);

        assert_eq!(bits.collect_vec(), bv.into_iter().collect_vec());
    }

    #[test]
    fn simple_root() {
        let siblings = (0..5u64).map(Element::from).collect::<Vec<_>>();
        let path = MerklePath::<6> {
            siblings: siblings.clone(),
        };

        let root = path.compute_root(Element::from(0u64));

        // because 0 is the lowest (left-most) possible value, every merge is this way round
        let expected_root = siblings.into_iter().fold(Element::from(0u64), hmerge);

        assert_eq!(root, expected_root);
    }
}
