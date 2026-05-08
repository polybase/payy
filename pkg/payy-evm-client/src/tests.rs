use async_trait::async_trait;
use element::Element;
use payy_evm_client_interface::{
    ClaimSourceKind, IncomingTransfer, PayyEvmCallRequest, PayyEvmLog, PayyEvmLogFilter,
    PayyEvmReadClient, PayyNetworkPreset, PayyTransactionReceipt,
    PayyWaitForTransactionReceiptArgs, PrivacyAddress, Result, TxHash,
};
use serde_json::Value;
use zk_primitives::EvmNote;

use crate::links::{encode_direct_claim_link, encode_ephemeral_claim_link};
use crate::{LinksClient, LocalPrivacySigner};

mod bridge;
mod client;
mod state;

struct MockReadClient {
    chain_id: u64,
    block_number: u64,
    read_response: Vec<u8>,
}

#[async_trait]
impl PayyEvmReadClient for MockReadClient {
    async fn get_chain_id(&self) -> Result<u64> {
        Ok(self.chain_id)
    }

    async fn get_block_number(&self) -> Result<u64> {
        Ok(self.block_number)
    }

    async fn read_contract(&self, _request: PayyEvmCallRequest) -> Result<Vec<u8>> {
        Ok(self.read_response.clone())
    }

    async fn get_logs(&self, _filter: PayyEvmLogFilter) -> Result<Vec<PayyEvmLog>> {
        Ok(Vec::new())
    }

    async fn get_transaction_receipt(
        &self,
        _hash: TxHash,
    ) -> Result<Option<PayyTransactionReceipt>> {
        Ok(None)
    }

    async fn wait_for_transaction_receipt(
        &self,
        _args: PayyWaitForTransactionReceiptArgs,
    ) -> Result<PayyTransactionReceipt> {
        Err(payy_evm_client_interface::Error::MissingCapability {
            capability: "receipt",
        })
    }
}

#[test]
fn network_preset_resolution_returns_bridge_config() {
    let config = PayyNetworkPreset::Dev.config();

    assert_eq!(config.chain_id, 7297);
    assert_eq!(config.privacy_bridge[0], 0x31);
}

#[test]
fn base_links_parse_v3_direct_payloads() {
    let note = EvmNote {
        kind: Element::ONE,
        token: Element::ONE,
        nonce: Element::ZERO,
        psi: Element::ONE,
        owner: Element::ONE,
        value: Element::ONE,
    };
    let link = encode_direct_claim_link(note, Some("hello"));
    let parsed = LinksClient.parse(&link.value).unwrap();

    assert_eq!(parsed.message.as_deref(), Some("hello"));
    assert_eq!(parsed.claim_source_kind, ClaimSourceKind::Direct);
    assert!(parsed.direct_note.is_some());
}

#[test]
fn shared_v3_fixture_matches_rust_crypto_and_links() {
    let fixture: Value =
        serde_json::from_str(include_str!("../../../fixtures/payy-evm-client/v3.json")).unwrap();
    assert_eq!(
        PayyNetworkPreset::Dev.config().chain_id,
        fixture["network_presets"]["dev"]["chain_id"]
            .as_u64()
            .unwrap()
    );
    assert_eq!(
        format!(
            "0x{}",
            hex::encode(PayyNetworkPreset::Dev.config().privacy_bridge)
        ),
        fixture["network_presets"]["dev"]["privacy_bridge"]
            .as_str()
            .unwrap()
    );
    let private_key = parse_b256(fixture["grumpkin_private_key"].as_str().unwrap());
    let signer = LocalPrivacySigner::from_grumpkin_private_key(private_key).unwrap();
    let address = signer.privacy_address();
    let expected_address = PrivacyAddress::new(parse_b256(
        fixture["privacy_address"]["bytes"].as_str().unwrap(),
    ));

    assert_eq!(address, expected_address);
    assert_eq!(
        format!("0x{}", hex::encode(address.prefix().unwrap().bytes)),
        fixture["privacy_address"]["prefix6"].as_str().unwrap()
    );
    assert_eq!(
        format!("0x{}", hex::encode(address.owner().unwrap().to_be_bytes())),
        fixture["privacy_address"]["owner"].as_str().unwrap()
    );

    let leading_private_key = parse_b256(
        fixture["leading_zero_prefix"]["grumpkin_private_key"]
            .as_str()
            .unwrap(),
    );
    let leading_signer =
        LocalPrivacySigner::from_grumpkin_private_key(leading_private_key).unwrap();
    let leading_address = leading_signer.privacy_address();

    assert_eq!(
        format!("0x{}", hex::encode(leading_address.bytes)),
        fixture["leading_zero_prefix"]["privacy_address"]
            .as_str()
            .unwrap()
    );
    assert_eq!(
        format!("0x{}", hex::encode(leading_address.prefix().unwrap().bytes)),
        fixture["leading_zero_prefix"]["prefix6"].as_str().unwrap()
    );
    assert_eq!(
        format!(
            "0x{}",
            hex::encode(leading_address.owner().unwrap().to_be_bytes())
        ),
        fixture["leading_zero_prefix"]["owner"].as_str().unwrap()
    );

    let note = EvmNote {
        kind: Element::ONE,
        token: Element::ONE,
        nonce: Element::ZERO,
        psi: Element::ONE,
        owner: address.owner().unwrap(),
        value: Element::ONE,
    };
    let direct_link = encode_direct_claim_link(note, Some("hello world"));
    let ephemeral_link = encode_ephemeral_claim_link(
        &IncomingTransfer {
            note,
            commitment: parse_b256(fixture["note"]["commitment"].as_str().unwrap()),
            ephemeral_private_key: private_key,
            source_tx_hash: None,
            source_bridge_tx_hash: None,
        },
        Some("handoff"),
    );

    assert_eq!(
        direct_link.value,
        fixture["direct_send_delivery"]["link"].as_str().unwrap()
    );
    assert_eq!(
        ephemeral_link.value,
        fixture["incoming_transfer"]["link"].as_str().unwrap()
    );

    let direct = LinksClient
        .parse(fixture["direct_send_delivery"]["link"].as_str().unwrap())
        .unwrap();
    let ephemeral = LinksClient
        .parse(fixture["incoming_transfer"]["link"].as_str().unwrap())
        .unwrap();

    assert_eq!(direct.claim_source_kind, ClaimSourceKind::Direct);
    assert_eq!(ephemeral.claim_source_kind, ClaimSourceKind::Ephemeral);
    assert_eq!(
        direct.direct_note.unwrap().commitment,
        parse_b256(fixture["note"]["commitment"].as_str().unwrap())
    );
}

fn parse_b256(value: &str) -> [u8; 32] {
    let raw = value.strip_prefix("0x").unwrap();
    let mut bytes = [0u8; 32];
    hex::decode_to_slice(raw, &mut bytes).unwrap();
    bytes
}
