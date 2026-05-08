use std::sync::Arc;

use async_trait::async_trait;
use element::Element;
use hash::compute_merkle_root;
use payy_evm_client_interface::{
    B256, EphemeralKeyPair, Error, OwnedNote, OwnerSignature, PaddingInputNote, PayyEvmCallRequest,
    PayyEvmLog, PayyEvmLogFilter, PayyEvmReadClient, PayyNetworkPreset, PayyTransactionReceipt,
    PayyWaitForTransactionReceiptArgs, PrivacyAccount, PrivacyOperationKind, PrivacySigner,
    RealInputNote, ResolvedInputNote, Result, TxHash, TxnData,
    grumpkin_public_key_from_private_key,
};
use payy_evm_client_prover_interface::{PrivacyCircuit, PrivacyProver, ProveOutput, ProveRequest};
use zk_primitives::EvmNote;

use crate::util::{address_to_element, element_to_b256};
use crate::{BaseClient, EphemeralSendParams, LocalPrivacySigner};

struct EphemeralReadClient {
    chain_id: u64,
    chain_public_key: (B256, B256),
}

#[async_trait]
impl PayyEvmReadClient for EphemeralReadClient {
    async fn get_chain_id(&self) -> Result<u64> {
        Ok(self.chain_id)
    }

    async fn get_block_number(&self) -> Result<u64> {
        Ok(11)
    }

    async fn read_contract(&self, request: PayyEvmCallRequest) -> Result<Vec<u8>> {
        if request.data.len() == 4 {
            let mut out = Vec::with_capacity(64);
            out.extend_from_slice(&self.chain_public_key.0);
            out.extend_from_slice(&self.chain_public_key.1);
            return Ok(out);
        }
        Ok(vec![0; 32])
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
        Err(Error::MissingCapability {
            capability: "receipt",
        })
    }
}

struct FailingPrivacySigner;

impl PrivacySigner for FailingPrivacySigner {
    fn accounts(&self) -> Result<Vec<PrivacyAccount>> {
        Err(global_signer_used())
    }

    fn sign_tx_commitment(
        &self,
        _privacy_account: PrivacyAccount,
        _tx_commitment: B256,
    ) -> Result<OwnerSignature> {
        Err(global_signer_used())
    }

    fn decrypt_sender_note(
        &self,
        _privacy_account: PrivacyAccount,
        _txn_data: TxnData,
    ) -> Result<Option<EvmNote>> {
        Err(global_signer_used())
    }

    fn decrypt_recipient_note(
        &self,
        _privacy_account: PrivacyAccount,
        _txn_data: TxnData,
    ) -> Result<Option<EvmNote>> {
        Err(global_signer_used())
    }

    fn generate_ephemeral_key(&self) -> Result<EphemeralKeyPair> {
        Err(global_signer_used())
    }
}

fn global_signer_used() -> Error {
    Error::MissingCapability {
        capability: "global_privacy_signer",
    }
}

struct MockProver;

#[async_trait]
impl PrivacyProver for MockProver {
    async fn prove(
        &self,
        request: ProveRequest,
    ) -> payy_evm_client_prover_interface::Result<ProveOutput> {
        assert_eq!(request.circuit, PrivacyCircuit::TransferSend);
        Ok(ProveOutput {
            verification_key_hash: [0x91; 32],
            proof: vec![0x92; 64],
            public_inputs: vec![[0x93; 32]; 33],
        })
    }
}

#[tokio::test]
async fn ephemeral_send_uses_signer_backed_account_for_key_generation() -> Result<()> {
    let config = PayyNetworkPreset::Dev.config();
    let signer = Arc::new(LocalPrivacySigner::from_grumpkin_private_key([0x11; 32])?);
    let privacy_address = signer.privacy_address();
    let chain_key = grumpkin_public_key_from_private_key([0x22; 32])?;
    let read_client = Arc::new(EphemeralReadClient {
        chain_id: config.chain_id,
        chain_public_key: (element_to_b256(chain_key.0), element_to_b256(chain_key.1)),
    });
    let client = BaseClient::builder(config, read_client)
        .prover(Arc::new(MockProver))
        .build()
        .privacy_signer(Arc::new(FailingPrivacySigner))
        .privacy();
    let mut accounts = signer.accounts()?;
    let account = accounts.remove(0);
    let token = [0x44; 20];
    let note = EvmNote {
        kind: Element::ONE,
        token: address_to_element(token),
        nonce: Element::ZERO,
        psi: Element::ONE,
        owner: privacy_address.owner()?,
        value: Element::from(10u64),
    };
    let merkle_path = [Element::ZERO; 160];
    let recent_root = compute_merkle_root(note.commitment(), note.commitment(), &merkle_path);
    let input = ResolvedInputNote::Real(Box::new(RealInputNote {
        owned_note: OwnedNote {
            note,
            commitment: element_to_b256(note.commitment()),
            nullifier: element_to_b256(note.nullifier()),
            nonce_hash: [0; 32],
            source_block: None,
            source_tx_hash: None,
            source_bridge_tx_hash: None,
        },
        merkle_path: Vec::new(),
        recent_root: element_to_b256(recent_root),
    }));

    let prepared = Box::pin(
        client
            .send()
            .ephemeral(EphemeralSendParams {
                privacy_account: account,
                token,
                amount: Element::ONE,
                bridge_memo: None,
            })
            .with_owned_input(ResolvedInputNote::Padding(PaddingInputNote {
                recent_root: element_to_b256(recent_root),
            }))
            .with_owned_input(input)
            .prepare(),
    )
    .await?;

    assert_eq!(
        prepared.prepared_call().operation,
        PrivacyOperationKind::TransferSend
    );
    assert_eq!(
        prepared.payload().source_bridge_tx_hash,
        Some(client.bridge().compute_tx_hash(
            prepared.prepared_call().verification_key_hash,
            &prepared.prepared_call().proof,
            &prepared.prepared_call().public_inputs,
        ))
    );
    Ok(())
}
