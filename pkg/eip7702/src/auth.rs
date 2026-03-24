use crate::types::Authorization;
use ethereum_types::{Address, H256, U256};
use rlp::RlpStream;
use secp256k1::{
    Message, PublicKey, Secp256k1, SecretKey, ecdsa::RecoverableSignature, ecdsa::RecoveryId,
};
use sha3::{Digest, Keccak256};

fn u256_be_trim(v: &U256) -> Vec<u8> {
    if v.is_zero() {
        return Vec::new();
    }
    let mut buf = [0u8; 32];
    v.to_big_endian(&mut buf);
    let first = buf.iter().position(|&b| b != 0).unwrap_or(31);
    buf[first..].to_vec()
}

/// keccak256( 0x05 || rlp([chain_id, delegate, nonce]) )
pub fn auth_message_hash(chain_id: U256, delegate: Address, nonce: U256) -> H256 {
    let mut s = RlpStream::new_list(3);

    s.append(&u256_be_trim(&chain_id));
    s.append(&delegate.as_bytes());
    s.append(&u256_be_trim(&nonce));

    let out = s.out();

    let mut pre = Vec::with_capacity(1 + out.len());
    pre.push(0x05);
    pre.extend_from_slice(&out);

    H256::from_slice(Keccak256::digest(&pre).as_slice())
}

/// Sign authorization tuple with EOA secret key.
pub fn sign_authorization(
    sk: &SecretKey,
    chain_id: U256,
    delegate: Address,
    nonce: U256,
) -> Authorization {
    let digest = auth_message_hash(chain_id, delegate, nonce);
    let secp = Secp256k1::new();

    let sig: RecoverableSignature =
        secp.sign_ecdsa_recoverable(&Message::from_digest_slice(digest.as_bytes()).unwrap(), sk);

    let (rid, b) = sig.serialize_compact();

    Authorization {
        chain_id,
        delegate,
        nonce,
        y_parity: (rid.to_i32() as u8) & 1,
        r: H256::from_slice(&b[0..32]),
        s: H256::from_slice(&b[32..64]),
    }
}

/// Recover the EOA address that produced the Authorization signature.
pub fn recover_authority(auth: &Authorization) -> Address {
    let digest = auth_message_hash(auth.chain_id, auth.delegate, auth.nonce);

    let mut sig_bytes = [0u8; 64];
    sig_bytes[..32].copy_from_slice(auth.r.as_bytes());
    sig_bytes[32..].copy_from_slice(auth.s.as_bytes());

    let rid = RecoveryId::from_i32((auth.y_parity & 1) as i32).expect("valid y_parity");
    let rec_sig = RecoverableSignature::from_compact(&sig_bytes, rid).expect("valid sig");

    let pk = Secp256k1::new()
        .recover_ecdsa(
            &Message::from_digest_slice(digest.as_bytes()).unwrap(),
            &rec_sig,
        )
        .expect("recover public key");

    public_key_to_address(&pk)
}

fn public_key_to_address(pk: &PublicKey) -> Address {
    let uncompressed = pk.serialize_uncompressed();
    let hash = Keccak256::digest(&uncompressed[1..]);

    let mut addr = [0u8; 20];
    addr.copy_from_slice(&hash[12..]);

    Address::from(addr)
}

#[cfg(test)]
mod tests {
    use super::*;
    use secp256k1::SecretKey;

    #[test]
    fn sign_and_recover_matches() {
        // Deterministic secret key
        let sk = SecretKey::from_slice(&[1u8; 32]).unwrap();
        // Random-ish values
        let chain_id = U256::from(1u64);
        let delegate = Address::from_low_u64_be(2);
        let nonce = U256::from(3u64);
        let auth = sign_authorization(&sk, chain_id, delegate, nonce);
        let recovered = recover_authority(&auth);
        // Derive expected address from the secret key's public key
        let secp = Secp256k1::new();
        let pk = secp256k1::PublicKey::from_secret_key(&secp, &sk);
        let expected = public_key_to_address(&pk);
        assert_eq!(recovered, expected);
    }
}
