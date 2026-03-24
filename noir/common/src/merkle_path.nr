use poseidon::poseidon2;
use crate::MerklePath;

impl MerklePath {
    pub fn root_from_leaf(self, leaf: Field) -> Field {
        let bits: [u1; 254] = leaf.to_le_bits();
        let mut hash = leaf;
        for i in 0..160 {
            let sibling = self.path[i];
            if bits[i] == 0 {
                hash = poseidon2::Poseidon2::hash([hash, sibling], 2);
            } else {
                hash = poseidon2::Poseidon2::hash([sibling, hash], 2);
            }
        }

        hash
    }

    pub fn validate(self, leaf: Field, root: Field) -> bool {
        self.root_from_leaf(leaf) == root
    }
}
