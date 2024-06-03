use crate::{
    constants::{UTXO_INPUTS, UTXO_OUTPUTS},
    data::{ParameterSet, SnarkWitness, SnarkWitnessV1, UTXOProof, Utxo},
    CircuitKind, Snark,
};
use primitives::hash::CryptoHash;
use sha3::{Digest, Keccak256};
use zk_primitives::Element;

impl<const MERKLE_D: usize> UTXOProof<MERKLE_D> {
    pub fn new(
        recent_root: Element,
        mb_hash: Element,
        mb_value: Element,
        input_leaves: [Element; UTXO_INPUTS],
        output_leaves: [Element; UTXO_OUTPUTS],
        proof: Vec<u8>,
    ) -> Self {
        Self {
            recent_root,
            mb_hash,
            mb_value,
            input_leaves,
            output_leaves,
            proof,
        }
    }

    pub fn hash(&self) -> CryptoHash {
        let mut sorted_input_leaves = self.input_leaves;
        sorted_input_leaves.sort();
        let mut sorted_output_leaves = self.output_leaves;
        sorted_output_leaves.sort();

        let mut hasher = Keccak256::new();
        hasher.update(self.recent_root.to_be_bytes());
        hasher.update(self.mb_hash.to_be_bytes());
        hasher.update(self.mb_value.to_be_bytes());
        for leaf in sorted_input_leaves.iter() {
            hasher.update(leaf.to_be_bytes());
        }
        for leaf in sorted_output_leaves.iter() {
            hasher.update(leaf.to_be_bytes());
        }

        CryptoHash::new(hasher.finalize().into())
    }

    /// Whether this UTXO is a mint or burn.
    ///
    /// If `true`, this is a mint or burn, otherwise it is a transfer
    pub fn is_mint_or_burn(&self) -> bool {
        self.mb_hash != Element::NULL_HASH || self.mb_value != Element::NULL_HASH
    }

    pub fn is_mint(&self) -> bool {
        self.is_mint_or_burn() && self.input_leaves.iter().all(|l| *l == Element::NULL_HASH)
    }

    pub fn is_burn(&self) -> bool {
        self.is_mint_or_burn() && self.output_leaves.iter().all(|l| *l == Element::NULL_HASH)
    }

    pub fn from_snark_witness(snark: SnarkWitness) -> Self {
        let SnarkWitness::V1(snark) = snark;
        let instances = &snark.instances[0];
        let recent_root = instances[0];
        let mb_hash = instances[1];
        let mb_value = instances[2];
        let input_leaves = [instances[3], instances[4]];
        let output_leaves = [instances[5], instances[6]];
        Self {
            recent_root,
            mb_hash,
            mb_value,
            input_leaves,
            output_leaves,
            proof: snark.proof,
        }
    }

    pub fn to_snark(&self, params: ParameterSet) -> Snark {
        let utxo = Utxo::<MERKLE_D>::default();
        let (_, vk) = utxo.keygen(params);

        match self.to_snark_witness() {
            SnarkWitness::V1(witness) => witness.to_snark(&vk, params),
        }
    }

    pub fn to_snark_witness(&self) -> SnarkWitness {
        let sw = SnarkWitnessV1::new(vec![self.instances()], self.proof.clone());
        SnarkWitness::V1(sw)
    }

    pub fn leaves(&self) -> Vec<Element> {
        self.input_leaves
            .into_iter()
            .chain(self.output_leaves.into_iter())
            .collect()
    }

    pub fn instances(&self) -> Vec<Element> {
        vec![self.recent_root, self.mb_hash, self.mb_value]
            .into_iter()
            .chain(self.leaves())
            .collect()
    }

    pub fn verify(&self) -> bool {
        match self.to_snark_witness() {
            SnarkWitness::V1(sw) => sw.verify(CircuitKind::Utxo),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        constants::MERKLE_TREE_DEPTH,
        data::{InputNote, Note, UtxoKind},
    };

    use super::*;

    #[test]
    fn gen_utxo() {
        let u = Utxo::<161>::new(
            [InputNote::padding_note(), InputNote::padding_note()],
            [Note::padding_note(), Note::padding_note()],
            smirk::Tree::<161, ()>::new().root_hash(),
            UtxoKind::Transfer,
        );

        let snark = u.snark(CircuitKind::Utxo).unwrap();

        assert!(snark.to_witness().verify(CircuitKind::Utxo));

        let utxo_proof = UTXOProof::<161>::from_snark_witness(SnarkWitness::V1(snark.to_witness()));
        assert!(utxo_proof.verify());

        let snark_witness = utxo_proof.to_snark_witness();
        println!("{}", serde_json::to_string(&snark_witness).unwrap());
    }

    #[test]
    fn bench_txn_hashing() {
        let txn = UTXOProof::<MERKLE_TREE_DEPTH>::new(
            Element::new(1),
            Element::new(2),
            Element::new(3),
            [Element::new(5), Element::new(6)],
            [Element::new(7), Element::new(8)],
            vec![],
        );

        let start = std::time::Instant::now();
        for _ in 0..1000 {
            let _ = txn.hash();
        }

        eprintln!("1000 hashes took {:?}", start.elapsed());
    }
}
