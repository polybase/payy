use common::Address;

pub struct Signature {
    pub r: [u8; 32],
    pub s: [u8; 32],

    pub sender_pubkey_x: [u8; 32],
    pub sender_pubkey_y: [u8; 32],
}

impl Signature {
    pub fn matches_transaction_hash(self, hash: [u8; 32]) -> bool {
        std::ecdsa_secp256k1::verify_signature(
            self.sender_pubkey_x,
            self.sender_pubkey_y,
            self.r.concat(self.s),
            hash,
        )
    }

    pub fn sender_address(self) -> Address {
        let hash = keccak256::keccak256(self.sender_pubkey_x.concat(self.sender_pubkey_y), 64);
        common::slice(hash, 12)
    }
}
