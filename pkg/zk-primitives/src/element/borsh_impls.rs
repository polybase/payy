use borsh::{BorshDeserialize, BorshSerialize};
use ethnum::U256;

use super::Element;

impl BorshSerialize for Element {
    fn serialize<W: std::io::Write>(&self, writer: &mut W) -> std::io::Result<()> {
        self.0.to_be_bytes().serialize(writer)
    }
}

impl BorshDeserialize for Element {
    fn deserialize_reader<R: std::io::Read>(reader: &mut R) -> std::io::Result<Self> {
        let bytes = <[u8; 32]>::deserialize_reader(reader)?;
        Ok(Self(U256::from_be_bytes(bytes)))
    }
}
