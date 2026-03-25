mod abi_family;
mod bytes_family;
mod hash_family;
mod numeric_family;
pub(crate) mod render;
mod rlp_family;

use crate::{cli::util::UtilAction, error::Result, output::OutputMode};

pub fn run(output_mode: OutputMode, action: UtilAction) -> Result<()> {
    match action {
        UtilAction::AbiEncode(_)
        | UtilAction::AbiEncodeEvent(_)
        | UtilAction::Calldata(_)
        | UtilAction::DecodeAbi(_)
        | UtilAction::DecodeCalldata(_)
        | UtilAction::DecodeError(_)
        | UtilAction::DecodeEvent(_)
        | UtilAction::DecodeString(_) => abi_family::run(output_mode, action),
        UtilAction::AddressZero
        | UtilAction::ConcatHex(_)
        | UtilAction::FormatBytes32String(_)
        | UtilAction::FromBin(_)
        | UtilAction::FromUtf8(_)
        | UtilAction::HashZero
        | UtilAction::Pad(_)
        | UtilAction::ParseBytes32Address(_)
        | UtilAction::ParseBytes32String(_)
        | UtilAction::PrettyCalldata(_)
        | UtilAction::ToAscii(_)
        | UtilAction::ToBytes32(_)
        | UtilAction::ToHexdata(_)
        | UtilAction::ToUtf8(_) => bytes_family::run(output_mode, action),
        UtilAction::ComputeAddress(_)
        | UtilAction::Create2(_)
        | UtilAction::HashMessage(_)
        | UtilAction::Index(_)
        | UtilAction::IndexErc7201(_)
        | UtilAction::Keccak(_)
        | UtilAction::Namehash(_)
        | UtilAction::Sig(_)
        | UtilAction::SigEvent(_)
        | UtilAction::ToCheckSumAddress(_) => hash_family::run(output_mode, action),
        UtilAction::FromRlp(_) | UtilAction::ToRlp(_) => rlp_family::run(output_mode, action),
        _ => numeric_family::run(output_mode, action),
    }
}
