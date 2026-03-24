// lint-long-file-override allow-max-lines=300
use crate::types::Authorization;
use ethereum_types::{Address, H256, U256};
use rlp::RlpStream;
use secp256k1::{Message, Secp256k1, SecretKey};
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

fn trim_leading_zeros(bytes: &[u8]) -> Vec<u8> {
    if bytes.iter().all(|&b| b == 0) {
        return Vec::new();
    }

    let first = bytes
        .iter()
        .position(|&b| b != 0)
        .unwrap_or(bytes.len() - 1);
    bytes[first..].to_vec()
}

/// Construct the delegation indicator code bytes: 0xef0100 || address (20 bytes)
pub fn delegation_indicator_code(delegate: Address) -> Vec<u8> {
    let mut code = Vec::with_capacity(3 + 20);
    code.extend_from_slice(&[0xef, 0x01, 0x00]);
    code.extend_from_slice(delegate.as_bytes());

    code
}

/// Build a JSON object representing a typed-0x04 SetCode transaction payload
/// including authorizationList, suitable for `eth_sendTransaction`.
#[allow(clippy::too_many_arguments)]
pub fn build_set_code_tx_json(
    from: Address,
    to: Address,
    chain_id: U256,
    nonce: U256,
    gas: U256,
    max_fee_per_gas: U256,
    max_priority_fee_per_gas: U256,
    authorization: &Authorization,
) -> serde_json::Value {
    fn fmt_u256(v: U256) -> String {
        if v.is_zero() {
            "0x0".to_string()
        } else {
            format!("0x{v:x}")
        }
    }

    // Build object while conditionally including fields that must not be zeroed
    let mut obj = serde_json::Map::new();
    obj.insert("type".into(), serde_json::Value::String("0x4".into()));
    obj.insert(
        "from".into(),
        serde_json::Value::String(format!("0x{from:x}")),
    );
    obj.insert("to".into(), serde_json::Value::String(format!("0x{to:x}")));
    obj.insert(
        "chainId".into(),
        serde_json::Value::String(fmt_u256(chain_id)),
    );
    obj.insert("nonce".into(), serde_json::Value::String(fmt_u256(nonce)));
    // Do NOT include gas when zero; clients interpret explicit 0 as allowance=0 during estimation
    if !gas.is_zero() {
        obj.insert("gas".into(), serde_json::Value::String(fmt_u256(gas)));
    }
    obj.insert(
        "maxFeePerGas".into(),
        serde_json::Value::String(fmt_u256(max_fee_per_gas)),
    );
    obj.insert(
        "maxPriorityFeePerGas".into(),
        serde_json::Value::String(fmt_u256(max_priority_fee_per_gas)),
    );
    obj.insert("data".into(), serde_json::Value::String("0x".into()));

    let auth = serde_json::json!({
        "chainId": fmt_u256(authorization.chain_id),
        "address": format!("0x{:x}", authorization.delegate),
        "nonce": fmt_u256(authorization.nonce),
        "yParity": format!("0x{:x}", authorization.y_parity),
        "r": format!("0x{:x}", authorization.r),
        "s": format!("0x{:x}", authorization.s),
    });
    obj.insert(
        "authorizationList".into(),
        serde_json::Value::Array(vec![auth]),
    );
    obj.insert("accessList".into(), serde_json::Value::Array(vec![]));

    serde_json::Value::Object(obj)
}

/// Sign a type-0x02 EIP-1559 transaction envelope with provided fields.
#[allow(clippy::too_many_arguments)]
pub fn sign_eip1559_envelope(
    chain_id: U256,
    nonce: U256,
    max_priority_fee_per_gas: U256,
    max_fee_per_gas: U256,
    gas: U256,
    to: Address,
    value: U256,
    data: Vec<u8>,
    sk: &SecretKey,
) -> Vec<u8> {
    let mut rlp = RlpStream::new_list(9);
    rlp.append(&u256_be_trim(&chain_id));
    rlp.append(&u256_be_trim(&nonce));
    rlp.append(&u256_be_trim(&max_priority_fee_per_gas));
    rlp.append(&u256_be_trim(&max_fee_per_gas));
    rlp.append(&u256_be_trim(&gas));
    rlp.append(&to.as_bytes());
    rlp.append(&u256_be_trim(&value));
    rlp.append(&data);

    let access_list = RlpStream::new_list(0);
    rlp.append_raw(&access_list.out(), 1);

    let payload = rlp.out();

    let mut pre = Vec::with_capacity(1 + payload.len());
    pre.push(0x02);
    pre.extend_from_slice(&payload);

    let sighash = H256::from_slice(Keccak256::digest(&pre).as_slice());

    let sig = Secp256k1::new()
        .sign_ecdsa_recoverable(&Message::from_digest_slice(sighash.as_bytes()).unwrap(), sk);
    let (rid, sig_bytes) = sig.serialize_compact();
    let y_parity = (rid.to_i32() as u8) & 1;

    let mut final_rlp = RlpStream::new_list(12);
    final_rlp.append(&u256_be_trim(&chain_id));
    final_rlp.append(&u256_be_trim(&nonce));
    final_rlp.append(&u256_be_trim(&max_priority_fee_per_gas));
    final_rlp.append(&u256_be_trim(&max_fee_per_gas));
    final_rlp.append(&u256_be_trim(&gas));
    final_rlp.append(&to.as_bytes());
    final_rlp.append(&u256_be_trim(&value));
    final_rlp.append(&data);

    let access_list2 = RlpStream::new_list(0);
    final_rlp.append_raw(&access_list2.out(), 1);
    final_rlp.append(&y_parity);
    final_rlp.append(&trim_leading_zeros(&sig_bytes[0..32]));
    final_rlp.append(&trim_leading_zeros(&sig_bytes[32..64]));

    let out = final_rlp.out();

    let mut raw = Vec::with_capacity(1 + out.len());
    raw.push(0x02);
    raw.extend_from_slice(&out);

    raw
}

/// Sign a type-0x04 SetCode transaction envelope with the provided Authorization tuple.
#[allow(clippy::too_many_arguments)]
pub fn sign_setcode_type4_envelope(
    chain_id: U256,
    nonce: U256,
    max_priority_fee_per_gas: U256,
    max_fee_per_gas: U256,
    gas: U256,
    user: Address,
    auth: &Authorization,
    sk: &SecretKey,
) -> Vec<u8> {
    let mut rlp = RlpStream::new_list(10);
    rlp.append(&u256_be_trim(&chain_id));
    rlp.append(&u256_be_trim(&nonce));
    rlp.append(&u256_be_trim(&max_priority_fee_per_gas));
    rlp.append(&u256_be_trim(&max_fee_per_gas));
    rlp.append(&u256_be_trim(&gas));
    rlp.append(&user.as_bytes());
    rlp.append(&u256_be_trim(&U256::zero()));
    rlp.append(&Vec::<u8>::new());

    let empty_access_list = RlpStream::new_list(0);
    rlp.append_raw(&empty_access_list.out(), 1);

    let mut auth_list = RlpStream::new_list(1);
    let mut tup = RlpStream::new_list(6);
    tup.append(&u256_be_trim(&auth.chain_id));
    tup.append(&auth.delegate.as_bytes());
    tup.append(&u256_be_trim(&auth.nonce));
    tup.append(&u64::from(auth.y_parity));
    tup.append(&trim_leading_zeros(auth.r.as_bytes()));
    tup.append(&trim_leading_zeros(auth.s.as_bytes()));
    auth_list.append_raw(&tup.out(), 1);
    rlp.append_raw(&auth_list.out(), 1);

    let payload = rlp.out();

    let mut pre = Vec::with_capacity(1 + payload.len());
    pre.push(0x04);
    pre.extend_from_slice(&payload);

    let sighash = H256::from_slice(Keccak256::digest(&pre).as_slice());
    let sig = Secp256k1::new()
        .sign_ecdsa_recoverable(&Message::from_digest_slice(sighash.as_bytes()).unwrap(), sk);
    let (rid, sig_bytes) = sig.serialize_compact();
    let y_parity = (rid.to_i32() as u8) & 1;

    let mut final_rlp = RlpStream::new_list(13);
    final_rlp.append(&u256_be_trim(&chain_id));
    final_rlp.append(&u256_be_trim(&nonce));
    final_rlp.append(&u256_be_trim(&max_priority_fee_per_gas));
    final_rlp.append(&u256_be_trim(&max_fee_per_gas));
    final_rlp.append(&u256_be_trim(&gas));
    final_rlp.append(&user.as_bytes());
    final_rlp.append(&u256_be_trim(&U256::zero()));
    final_rlp.append(&Vec::<u8>::new());

    let empty_access_list2 = RlpStream::new_list(0);
    final_rlp.append_raw(&empty_access_list2.out(), 1);

    let mut auth_list2 = RlpStream::new_list(1);
    let mut tup2 = RlpStream::new_list(6);
    tup2.append(&u256_be_trim(&auth.chain_id));
    tup2.append(&auth.delegate.as_bytes());
    tup2.append(&u256_be_trim(&auth.nonce));
    tup2.append(&u64::from(auth.y_parity));
    tup2.append(&trim_leading_zeros(auth.r.as_bytes()));
    tup2.append(&trim_leading_zeros(auth.s.as_bytes()));
    auth_list2.append_raw(&tup2.out(), 1);
    final_rlp.append_raw(&auth_list2.out(), 1);

    final_rlp.append(&y_parity);
    final_rlp.append(&trim_leading_zeros(&sig_bytes[0..32]));
    final_rlp.append(&trim_leading_zeros(&sig_bytes[32..64]));

    let out = final_rlp.out();
    let mut raw = Vec::with_capacity(1 + out.len());
    raw.push(0x04);
    raw.extend_from_slice(&out);

    raw
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn delegation_code_format() {
        let d = Address::from_low_u64_be(0xdeadbeef);
        let code = delegation_indicator_code(d);
        assert_eq!(code.len(), 23);
        assert_eq!(&code[0..3], &[0xef, 0x01, 0x00]);
        assert_eq!(&code[3..], d.as_bytes());
    }
}
