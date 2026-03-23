use aggregator_interface::Error;
use borsh::BorshSerialize;
use node_interface::BlockHeader;
use primitives::{block_height::BlockHeight, hash::CryptoHash, sig::Signature};
use sha3::{Digest, Keccak256};

#[derive(BorshSerialize)]
pub struct SerializableBlockHeader {
    pub height: BlockHeight,
    pub last_block_hash: CryptoHash,
    pub epoch_id: u64,
    pub last_final_block_hash: CryptoHash,
    pub approvals: Vec<Signature>,
}

impl From<&BlockHeader> for SerializableBlockHeader {
    fn from(header: &BlockHeader) -> Self {
        Self {
            height: header.height,
            last_block_hash: header.last_block_hash,
            epoch_id: header.epoch_id,
            last_final_block_hash: header.last_final_block_hash,
            approvals: header.approvals.clone(),
        }
    }
}

pub fn serialize_signatures(approvals: &[Signature]) -> Vec<Vec<u8>> {
    approvals.iter().map(|sig| sig.inner().to_vec()).collect()
}

pub fn block_header_hash(header: &BlockHeader) -> Result<[u8; 32], Error> {
    let serializable = SerializableBlockHeader::from(header);
    let mut bytes = Vec::new();
    serializable
        .serialize(&mut bytes)
        .map_err(|err| Error::ImplementationSpecific(Box::new(err)))?;
    let mut hasher = Keccak256::new();
    hasher.update(bytes);
    Ok(hasher.finalize().into())
}

pub fn map_node_error(err: node_interface::Error) -> Error {
    Error::ImplementationSpecific(Box::new(err))
}
