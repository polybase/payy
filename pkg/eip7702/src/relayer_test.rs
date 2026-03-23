use super::*;
use crate::txn::{delegation_indicator_code, sign_eip1559_envelope, sign_setcode_type4_envelope};
use crate::{auth_message_hash, sign_authorization};
use ethereum_types::{Address, H256};
use reqwest::Client;
use rlp::Rlp;
use testutil::eth::EthNode;

fn to_hex_u256(v: U256) -> String {
    if v.is_zero() {
        "0x0".to_string()
    } else {
        format!("0x{v:x}")
    }
}

async fn rpc_call(
    client: &Client,
    url: &str,
    method: &str,
    params: serde_json::Value,
) -> serde_json::Value {
    let req = serde_json::json!({"jsonrpc":"2.0","id":1,"method":method,"params":params});
    let res = client.post(url).json(&req).send().await.expect("rpc send");
    res.json().await.expect("rpc json")
}

#[tokio::test(flavor = "multi_thread")]
async fn auth_type04_sets_delegation_code_on_prague() {
    let eth = EthNode::default().run_and_deploy().await;
    let rpc_url = eth.rpc_url();
    let web3 = Web3::new(Http::new(&rpc_url).unwrap());
    let client = Client::new();

    // Keys: user (authorizer) and relayer (sender)
    let user_sk = SecretKey::from_slice(&[7u8; 32]).unwrap();
    let relayer_sk = SecretKey::from_slice(&[11u8; 32]).unwrap();
    let user_addr = eth_util::secret_key_to_address(&user_sk);
    let relayer_addr = eth_util::secret_key_to_address(&relayer_sk);

    // Fund both accounts
    let fund = U256::from_dec_str("100000000000000000000").unwrap();
    for addr in [user_addr, relayer_addr] {
        let params = serde_json::json!([format!("0x{addr:x}"), to_hex_u256(fund)]);
        let v = rpc_call(&client, &rpc_url, "hardhat_setBalance", params).await;
        assert!(v.get("error").is_none(), "setBalance error: {v}");
    }

    // Network and nonces
    let chain_id = web3.eth().chain_id().await.expect("chain id");
    let user_nonce = web3
        .eth()
        .transaction_count(user_addr, Some(web3::types::BlockNumber::Latest))
        .await
        .expect("user nonce");
    let relayer_nonce = web3
        .eth()
        .transaction_count(relayer_addr, Some(web3::types::BlockNumber::Pending))
        .await
        .expect("relayer nonce");

    // Sign Authorization for delegate target
    let delegate = Address::from_low_u64_be(0xdead_beef);
    let auth = sign_authorization(&user_sk, chain_id, delegate, user_nonce);

    // Fees suitable for Hardhat
    let gas = U256::from(200_000u64);
    let max_prio = U256::from(1_000_000_000u64);
    let max_fee = U256::from(2_000_000_000u64);

    // Sign raw type-0x04
    let raw = sign_setcode_type4_envelope(
        chain_id,
        relayer_nonce,
        max_prio,
        max_fee,
        gas,
        user_addr,
        &auth,
        &relayer_sk,
    );

    // Send raw and ensure success
    let send_res = rpc_call(
        &client,
        &rpc_url,
        "eth_sendRawTransaction",
        serde_json::json!([format!("0x{}", hex::encode(raw))]),
    )
    .await;
    let tx_hash = send_res
        .get("result")
        .and_then(|v| v.as_str())
        .unwrap_or_else(|| panic!("unexpected send response: {send_res}"));
    assert!(tx_hash.starts_with("0x"));

    // Verify code bytes reflect delegation indicator
    let expected_prefix = delegation_indicator_code(delegate);
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
    loop {
        let code = web3.eth().code(user_addr, None).await.expect("get code");
        if code.0.len() >= expected_prefix.len()
            && &code.0[0..expected_prefix.len()] == expected_prefix.as_slice()
        {
            break;
        }
        if std::time::Instant::now() > deadline {
            panic!(
                "delegation code not observed; got {:?}",
                hex::encode(code.0)
            );
        }
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    }

    let _digest: H256 = auth_message_hash(chain_id, delegate, user_nonce);
}

#[test]
fn execute_meta_payload_and_envelope_propagate_value() {
    let relayer_addr = Address::from_low_u64_be(0xabc);
    let user = Address::from_low_u64_be(0xdef);
    let calls = vec![
        MetaCall {
            target: Address::from_low_u64_be(1),
            value: U256::from(5u64),
            data: vec![0x01, 0x02],
        },
        MetaCall {
            target: Address::from_low_u64_be(2),
            value: U256::from(7u64),
            data: vec![0x03],
        },
    ];
    let nonce = U256::from(9u64);
    let valid_after = U256::from(1u64);
    let valid_until = U256::from(99u64);
    let signature = vec![0xaa, 0xbb, 0xcc];
    let call_data =
        encode_execute_meta_many_call(calls.clone(), nonce, valid_after, valid_until, signature);

    let payload = super::build_execute_meta_estimate_payload(relayer_addr, user, &call_data);
    let payload_value = payload
        .get("value")
        .and_then(|v| v.as_str())
        .expect("payload value hex");
    assert_eq!(payload_value, "0x0");

    let payload_data = payload
        .get("data")
        .and_then(|v| v.as_str())
        .expect("payload data hex");
    assert!(payload_data.starts_with("0x"));
    assert!(payload_data.ends_with(&hex::encode(&call_data)));

    let chain_id = U256::from(1u64);
    let tx_nonce = U256::from(3u64);
    let max_prio = U256::from(2u64);
    let max_fee = U256::from(4u64);
    let gas = U256::from(50_000u64);
    let sk = SecretKey::from_slice(&[5u8; 32]).unwrap();

    let raw = sign_eip1559_envelope(
        chain_id,
        tx_nonce,
        max_prio,
        max_fee,
        gas,
        user,
        U256::zero(),
        call_data,
        &sk,
    );

    assert_eq!(raw.first().copied(), Some(0x02));
    let rlp = Rlp::new(&raw[1..]);
    let value_bytes: Vec<u8> = rlp.val_at(6).expect("value field");
    let mut padded = [0u8; 32];
    let start = padded.len() - value_bytes.len();
    padded[start..].copy_from_slice(&value_bytes);
    let encoded_value = U256::from_big_endian(&padded);
    assert_eq!(encoded_value, U256::zero());
}
