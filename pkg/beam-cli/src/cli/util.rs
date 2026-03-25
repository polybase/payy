// lint-long-file-override allow-max-lines=280
use clap::{Args, Subcommand};

#[derive(Debug, Subcommand)]
pub enum UtilAction {
    /// Encode ABI arguments
    AbiEncode(AbiSignatureArgs),
    /// Encode ABI event arguments
    AbiEncodeEvent(AbiSignatureArgs),
    /// Print the zero address
    AddressZero,
    /// Build calldata from a function signature
    Calldata(AbiSignatureArgs),
    /// Compute a contract deployment address
    ComputeAddress(ComputeAddressArgs),
    /// Concatenate hex values
    ConcatHex(MultiValueArgs),
    /// Compute a CREATE2 contract address
    Create2(Create2Args),
    /// Decode ABI input or output data
    DecodeAbi(DecodeAbiArgs),
    /// Decode function calldata
    DecodeCalldata(SignatureDataArgs),
    /// Decode custom error data
    DecodeError(DecodeErrorArgs),
    /// Decode event data and topics
    DecodeEvent(DecodeEventArgs),
    /// Decode an ABI-encoded string
    DecodeString(InputValueArgs),
    /// Convert text to a bytes32 string
    FormatBytes32String(InputValueArgs),
    /// Format a value using a unit scale
    FormatUnits(ValueUnitArgs),
    /// Convert binary input to hex
    FromBin(InputValueArgs),
    /// Convert a fixed-point value to an integer
    FromFixedPoint(DecimalsValueArgs),
    /// Decode an RLP value
    FromRlp(FromRlpArgs),
    /// Convert UTF-8 text to hex
    FromUtf8(InputValueArgs),
    /// Format wei as decimal units
    FromWei(ValueUnitArgs),
    /// Hash a message with the Ethereum prefix
    HashMessage(InputValueArgs),
    /// Print the zero hash
    HashZero,
    /// Compute a mapping storage slot
    Index(IndexArgs),
    /// Compute an ERC-7201 storage slot
    IndexErc7201(InputValueArgs),
    /// Compute a keccak256 hash
    Keccak(InputValueArgs),
    /// Print the maximum signed integer
    MaxInt(IntegerTypeArgs),
    /// Print the maximum unsigned integer
    MaxUint(IntegerTypeArgs),
    /// Print the minimum signed integer
    MinInt(IntegerTypeArgs),
    /// Compute an ENS namehash
    Namehash(InputValueArgs),
    /// Pad hex data to a fixed length
    Pad(PadArgs),
    /// Decode an address from bytes32
    ParseBytes32Address(InputValueArgs),
    /// Decode text from bytes32
    ParseBytes32String(InputValueArgs),
    /// Parse decimal units into an integer
    ParseUnits(ValueUnitArgs),
    /// Pretty-print calldata
    PrettyCalldata(InputValueArgs),
    /// Shift a value left
    Shl(ShiftArgs),
    /// Shift a value right
    Shr(ShiftArgs),
    /// Compute a function selector
    Sig(InputValueArgs),
    /// Compute an event topic selector
    SigEvent(InputValueArgs),
    /// Convert hex data to ASCII
    ToAscii(InputValueArgs),
    /// Convert a value to another base
    ToBase(BaseConvertArgs),
    /// Convert a value to bytes32
    ToBytes32(InputValueArgs),
    /// Format an address with checksum casing
    ToCheckSumAddress(ChecksumArgs),
    /// Convert a value to decimal
    ToDec(BaseValueArgs),
    /// Convert an integer to fixed-point form
    ToFixedPoint(DecimalsValueArgs),
    /// Convert a value to hex
    ToHex(BaseValueArgs),
    /// Normalize input as hex data
    ToHexdata(InputValueArgs),
    /// Convert a value to int256
    ToInt256(SignedInputValueArgs),
    /// Encode a value as RLP
    ToRlp(InputValueArgs),
    /// Convert a value to uint256
    ToUint256(InputValueArgs),
    /// Convert a value to a named unit
    ToUnit(ValueUnitArgs),
    /// Convert hex data to UTF-8
    ToUtf8(InputValueArgs),
    /// Parse decimal units into wei
    ToWei(ValueUnitArgs),
}

#[derive(Clone, Debug, Args)]
pub struct AbiSignatureArgs {
    pub sig: String,
    pub args: Vec<String>,
}

#[derive(Clone, Debug, Args)]
pub struct BaseConvertArgs {
    #[arg(allow_hyphen_values = true)]
    pub value: Option<String>,
    pub base: Option<String>,
    #[arg(long = "base-in")]
    pub base_in: Option<String>,
}

#[derive(Clone, Debug, Args)]
pub struct BaseValueArgs {
    #[arg(allow_hyphen_values = true)]
    pub value: Option<String>,
    #[arg(long = "base-in")]
    pub base_in: Option<String>,
}

#[derive(Clone, Debug, Args)]
pub struct ChecksumArgs {
    pub address: Option<String>,
    pub chain_id: Option<String>,
}

#[derive(Clone, Debug, Args)]
pub struct ComputeAddressArgs {
    pub address: Option<String>,
    #[arg(long)]
    pub nonce: Option<String>,
    #[arg(long)]
    pub salt: Option<String>,
    #[arg(long = "init-code", conflicts_with = "init_code_hash")]
    pub init_code: Option<String>,
    #[arg(long = "init-code-hash", conflicts_with = "init_code")]
    pub init_code_hash: Option<String>,
}

#[derive(Clone, Debug, Args)]
pub struct Create2Args {
    #[arg(long)]
    pub deployer: Option<String>,
    #[arg(long)]
    pub salt: String,
    #[arg(long = "init-code", conflicts_with = "init_code_hash")]
    pub init_code: Option<String>,
    #[arg(long = "init-code-hash", conflicts_with = "init_code")]
    pub init_code_hash: Option<String>,
}

#[derive(Clone, Debug, Args)]
pub struct DecodeAbiArgs {
    pub sig: String,
    pub calldata: Option<String>,
    #[arg(short, long, default_value_t = false)]
    pub input: bool,
}

#[derive(Clone, Debug, Args)]
pub struct DecodeErrorArgs {
    #[arg(long)]
    pub sig: String,
    pub data: Option<String>,
}

#[derive(Clone, Debug, Args)]
pub struct DecodeEventArgs {
    #[arg(long)]
    pub sig: String,
    pub data: Option<String>,
    #[arg(long = "topic")]
    pub topics: Vec<String>,
}

#[derive(Clone, Debug, Args)]
pub struct DecimalsValueArgs {
    pub decimals: Option<String>,
    #[arg(allow_hyphen_values = true)]
    pub value: Option<String>,
}

#[derive(Clone, Debug, Args)]
pub struct FromRlpArgs {
    pub value: Option<String>,
    #[arg(long, default_value_t = false)]
    pub as_int: bool,
}

#[derive(Clone, Debug, Args)]
pub struct IndexArgs {
    pub key_type: String,
    pub key: String,
    pub slot_number: String,
}

#[derive(Clone, Debug, Args)]
pub struct InputValueArgs {
    pub value: Option<String>,
}

#[derive(Clone, Debug, Args)]
pub struct IntegerTypeArgs {
    pub ty: Option<String>,
}

#[derive(Clone, Debug, Args)]
pub struct MultiValueArgs {
    pub values: Vec<String>,
}

#[derive(Clone, Debug, Args)]
pub struct PadArgs {
    pub data: Option<String>,
    #[arg(long, default_value_t = 32)]
    pub len: usize,
    #[arg(long, default_value_t = false, conflicts_with = "left")]
    pub right: bool,
    #[arg(long, default_value_t = false, conflicts_with = "right")]
    pub left: bool,
}

#[derive(Clone, Debug, Args)]
pub struct ShiftArgs {
    #[arg(allow_hyphen_values = true)]
    pub value: String,
    pub bits: String,
    #[arg(long = "base-in")]
    pub base_in: Option<String>,
    #[arg(long = "base-out")]
    pub base_out: Option<String>,
}

#[derive(Clone, Debug, Args)]
pub struct SignatureDataArgs {
    pub sig: String,
    pub data: Option<String>,
}

#[derive(Clone, Debug, Args)]
pub struct SignedInputValueArgs {
    #[arg(allow_hyphen_values = true)]
    pub value: Option<String>,
}

#[derive(Clone, Debug, Args)]
pub struct ValueUnitArgs {
    #[arg(allow_hyphen_values = true)]
    pub value: Option<String>,
    pub unit: Option<String>,
}
