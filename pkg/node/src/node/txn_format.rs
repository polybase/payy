use borsh::{BorshDeserialize, BorshSerialize};
use primitives::block_height::BlockHeight;
use serde::{Deserialize, Serialize};
use wire_message::WireMessage;
use zk_primitives::UtxoProof;

#[derive(Debug, Clone, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct TxnMetadata {
    pub block_height: BlockHeight,
    pub block_time: Option<u64>,
    pub block_hash: [u8; 32],
    pub block_txn_index: u32,
}

#[derive(Debug, Clone)]
#[wire_message::wire_message]
pub enum TxnFormat {
    V1(UtxoProof, TxnMetadata),
    // TODO next version:
    // - cache the hash of the transaction in the metadata
}

impl WireMessage for TxnFormat {
    type Ctx = ();
    type Err = core::convert::Infallible;

    fn version(&self) -> u64 {
        match self {
            Self::V1(_, _) => 1,
        }
    }

    fn upgrade_once(self, _ctx: &mut Self::Ctx) -> Result<Self, wire_message::Error> {
        match self {
            Self::V1(_, _) => Err(Self::max_version_error()),
        }
    }
}

impl block_store::Transaction for TxnFormat {
    fn txn_hash(&self) -> [u8; 32] {
        match self {
            Self::V1(txn, _) => txn.hash().to_be_bytes(),
        }
    }

    fn input_elements(&self) -> Vec<element::Element> {
        match self {
            TxnFormat::V1(utxo_proof, _) => utxo_proof
                .public_inputs
                .input_commitments
                .iter()
                .filter(|c| !c.is_zero())
                .copied()
                .collect(),
        }
    }

    fn output_elements(&self) -> Vec<element::Element> {
        match self {
            TxnFormat::V1(utxo_proof, _) => utxo_proof
                .public_inputs
                .output_commitments
                .iter()
                .filter(|c| !c.is_zero())
                .copied()
                .collect(),
        }
    }

    fn mint_hash(&self) -> Option<element::Element> {
        match self {
            TxnFormat::V1(utxo_proof, _) => match utxo_proof.public_inputs.kind_messages() {
                zk_primitives::UtxoKindMessages::Mint(utxo_kind_mint_messages) => {
                    Some(utxo_kind_mint_messages.mint_hash)
                }
                zk_primitives::UtxoKindMessages::Burn(_) => None,
                zk_primitives::UtxoKindMessages::None => None,
            },
        }
    }
}
