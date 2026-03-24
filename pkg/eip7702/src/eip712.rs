use crate::types::MetaCall;
use ethereum_types::{Address, H256, U256};
use sha3::{Digest, Keccak256};
use web3::ethabi::{self, Token};

const EIP712DOMAIN_TYPEHASH: &str =
    "EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)";
const CALL_TYPEHASH: &str = "Call(address target,uint256 value,bytes data)";
// ExecuteMany uses pre-hashed calls as bytes32[], includes validity window
const EXECUTE_MANY_TYPEHASH: &str =
    "ExecuteMany(bytes32[] calls,uint256 nonce,uint256 validAfter,uint256 validUntil)";
const NAME: &str = "Eip7702SimpleAccount";
const VERSION: &str = "1";

fn keccak(bytes: impl AsRef<[u8]>) -> H256 {
    H256::from_slice(Keccak256::digest(bytes).as_slice())
}

/// EIP-712 domain separator for the SimpleAccount binding.
pub fn eip712_domain_separator(chain_id: U256, user: Address) -> H256 {
    keccak(ethabi::encode(&[
        Token::FixedBytes(keccak(EIP712DOMAIN_TYPEHASH).0.to_vec()),
        Token::FixedBytes(keccak(NAME).0.to_vec()),
        Token::FixedBytes(keccak(VERSION).0.to_vec()),
        Token::Uint(chain_id),
        Token::Address(user),
    ]))
}

fn hash_call(c: &MetaCall) -> H256 {
    keccak(ethabi::encode(&[
        Token::FixedBytes(keccak(CALL_TYPEHASH).0.to_vec()),
        Token::Address(c.target),
        Token::Uint(c.value),
        Token::FixedBytes(keccak(&c.data).0.to_vec()),
    ]))
}

/// Hash an array of calls exactly like the Solidity helper:
/// calls_hash = keccak256(abi.encodePacked(bytes32[] callHashes)) where
/// callHashes[i] = keccak256(abi.encode(CALL_TYPEHASH, target, value, keccak256(data)))
pub fn hash_calls(calls: &[MetaCall]) -> H256 {
    let mut packed = Vec::with_capacity(32 * calls.len());

    for c in calls {
        let h = hash_call(c);
        packed.extend_from_slice(h.as_bytes());
    }

    keccak(packed)
}

/// Compute EIP-712 digest for ExecuteMany(calls, nonce) on the user contract.
pub fn execute_meta_many_digest(
    chain_id: U256,
    user: Address,
    calls: &[MetaCall],
    nonce: U256,
    valid_after: U256,
    valid_until: U256,
) -> H256 {
    // EIP-712 array hashing: keccak256(abi.encodePacked(bytes32[]))
    let calls_hash = hash_calls(calls);

    let struct_hash = keccak(ethabi::encode(&[
        Token::FixedBytes(keccak(EXECUTE_MANY_TYPEHASH).0.to_vec()),
        Token::FixedBytes(calls_hash.0.to_vec()),
        Token::Uint(nonce),
        Token::Uint(valid_after),
        Token::Uint(valid_until),
    ]));

    let mut pre = Vec::with_capacity(66);
    pre.extend_from_slice(&[0x19, 0x01]);
    pre.extend_from_slice(eip712_domain_separator(chain_id, user).as_bytes());
    pre.extend_from_slice(struct_hash.as_bytes());

    keccak(pre)
}

/// ABI-encode executeMeta((address,uint256,bytes)[],uint256,uint256,uint256,bytes)
pub fn encode_execute_meta_many_call(
    calls: Vec<MetaCall>,
    nonce: U256,
    valid_after: U256,
    valid_until: U256,
    signature: Vec<u8>,
) -> Vec<u8> {
    let sel = {
        let mut h = Keccak256::new();
        h.update(b"executeMeta((address,uint256,bytes)[],uint256,uint256,uint256,bytes)");
        let out = h.finalize();
        [out[0], out[1], out[2], out[3]]
    };

    let call_tokens: Vec<Token> = calls
        .into_iter()
        .map(|c| {
            Token::Tuple(vec![
                Token::Address(c.target),
                Token::Uint(c.value),
                Token::Bytes(c.data),
            ])
        })
        .collect();

    let args = ethabi::encode(&[
        Token::Array(call_tokens),
        Token::Uint(nonce),
        Token::Uint(valid_after),
        Token::Uint(valid_until),
        Token::Bytes(signature),
    ]);

    let mut buf = Vec::with_capacity(4 + args.len());
    buf.extend_from_slice(&sel);
    buf.extend_from_slice(&args);

    buf
}

#[cfg(test)]
mod tests {
    use super::*;
    use hex::{FromHex, decode as hex_decode};

    #[test]
    fn domain_separator_is_32_bytes() {
        let d = eip712_domain_separator(U256::from(1u64), Address::zero());
        assert_eq!(d.as_bytes().len(), 32);
    }

    #[test]
    fn hash_calls_known_vector_matches_expected() {
        // calls: [(0x1111.., 1, 0x), (0x2222.., 2, 0xdeadbeef)]
        let a1 =
            Address::from_slice(&hex_decode("1111111111111111111111111111111111111111").unwrap());
        let a2 =
            Address::from_slice(&hex_decode("2222222222222222222222222222222222222222").unwrap());
        let calls = vec![
            MetaCall {
                target: a1,
                value: U256::from(1u64),
                data: vec![],
            },
            MetaCall {
                target: a2,
                value: U256::from(2u64),
                data: <Vec<u8>>::from_hex("deadbeef").unwrap(),
            },
        ];
        let h = hash_calls(&calls);
        // Load expected from shared fixtures at compile-time so it stays stable.
        // Path is relative to this source file.
        const EXPECTED_HEX: &str = include_str!("../../../fixtures/eip7702/hashcalls_vector1.hex");
        let trimmed = EXPECTED_HEX.trim().trim_start_matches("0x");
        let expected = H256::from_slice(&hex_decode(trimmed).unwrap());
        assert_eq!(h, expected);
    }
}
