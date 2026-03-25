// lint-long-file-override allow-max-lines=300
use contextful::{FromContextful, InternalError};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error, FromContextful)]
pub enum Error {
    #[error("[beam-cli] beam home directory not found")]
    BeamHomeNotFound,

    #[error("[beam-cli] wallet not found: {selector}")]
    WalletNotFound { selector: String },

    #[error("[beam-cli] wallet name cannot be empty or whitespace only")]
    WalletNameBlank,

    #[error("[beam-cli] wallet name cannot start with 0x: {name}")]
    WalletNameStartsWithAddressPrefix { name: String },

    #[error("[beam-cli] wallet name already exists: {name}")]
    WalletNameAlreadyExists { name: String },

    #[error("[beam-cli] wallet address already exists: {address}")]
    WalletAddressAlreadyExists { address: String },

    #[error("[beam-cli] wallet ENS name does not resolve to {address}: {name}")]
    WalletNameEnsAddressMismatch { address: String, name: String },

    #[error("[beam-cli] ens name not found: {name}")]
    EnsNameNotFound { name: String },

    #[error("[beam-cli] no default wallet configured")]
    NoDefaultWallet,

    #[error("[beam-cli] unknown chain: {chain}")]
    UnknownChain { chain: String },

    #[error("[beam-cli] invalid chain name: {name}")]
    InvalidChainName { name: String },

    #[error("[beam-cli] chain name already exists: {name}")]
    ChainNameAlreadyExists { name: String },

    #[error("[beam-cli] chain name conflicts with existing selector: {name}")]
    ChainNameConflictsWithSelector { name: String },

    #[error("[beam-cli] chain id already exists: {chain_id}")]
    ChainIdAlreadyExists { chain_id: u64 },

    #[error("[beam-cli] built-in chain cannot be removed: {chain}")]
    BuiltinChainRemovalNotAllowed { chain: String },

    #[error("[beam-cli] no rpc configured for chain: {chain}")]
    NoRpcConfigured { chain: String },

    #[error("[beam-cli] rpc already configured for {chain}: {rpc}")]
    RpcAlreadyExists { chain: String, rpc: String },

    #[error("[beam-cli] rpc not configured for {chain}: {rpc}")]
    RpcNotConfigured { chain: String, rpc: String },

    #[error("[beam-cli] at least one rpc must remain configured for {chain}")]
    ChainRequiresRpc { chain: String },

    #[error("[beam-cli] rpc chain id mismatch for {chain}: expected {expected}, got {actual}")]
    RpcChainIdMismatch {
        actual: u64,
        chain: String,
        expected: u64,
    },

    #[error("[beam-cli] token not configured on {chain}: {token}")]
    UnknownToken { chain: String, token: String },

    #[error("[beam-cli] token label cannot be empty or whitespace only")]
    TokenLabelBlank,

    #[error("[beam-cli] token label already exists on {chain}: {label}")]
    TokenLabelAlreadyExists { chain: String, label: String },

    #[error("[beam-cli] token already tracked on {chain}: {token}")]
    TokenAlreadyTracked { chain: String, token: String },

    #[error("[beam-cli] token not tracked on {chain}: {token}")]
    TokenNotTracked { chain: String, token: String },

    #[error("[beam-cli] native token is always tracked on {chain}")]
    NativeTokenAlwaysTracked { chain: String },

    #[error("[beam-cli] token label is reserved on {chain}: {label}")]
    ReservedTokenLabel { chain: String, label: String },

    #[error("[beam-cli] invalid private key")]
    InvalidPrivateKey,

    #[error("[beam-cli] invalid address: {value}")]
    InvalidAddress { value: String },

    #[error("[beam-cli] invalid transaction hash: {value}")]
    InvalidTransactionHash { value: String },

    #[error("[beam-cli] invalid block selector: {value}")]
    InvalidBlockSelector { value: String },

    #[error("[beam-cli] invalid rpc url: {value}")]
    InvalidRpcUrl { value: String },

    #[error("[beam-cli] invalid amount: {value}")]
    InvalidAmount { value: String },

    #[error("[beam-cli] unsupported decimals: {decimals} (max {max})")]
    UnsupportedDecimals { decimals: usize, max: usize },

    #[error("[beam-cli] missing input for beam util {command}")]
    MissingUtilInput { command: String },

    #[error("[beam-cli] prompt input closed while reading {label}")]
    PromptClosed { label: String },

    #[error("[beam-cli] invalid hex data: {value}")]
    InvalidHexData { value: String },

    #[error("[beam-cli] invalid utf-8 data")]
    InvalidUtf8Data,

    #[error("[beam-cli] invalid ascii data: {value}")]
    InvalidAsciiData { value: String },

    #[error("[beam-cli] invalid bytes32 value: {value}")]
    InvalidBytes32Value { value: String },

    #[error("[beam-cli] invalid integer type: {value}")]
    InvalidIntegerType { value: String },

    #[error("[beam-cli] invalid unit: {value}")]
    InvalidUnit { value: String },

    #[error("[beam-cli] invalid base: {value}")]
    InvalidBase { value: String },

    #[error("[beam-cli] invalid number: {value}")]
    InvalidNumber { value: String },

    #[error("[beam-cli] invalid bit count: {value}")]
    InvalidBitCount { value: String },

    #[error("[beam-cli] invalid rlp value: {value}")]
    InvalidRlpValue { value: String },

    #[error("[beam-cli] selector mismatch: expected {expected}, got {got}")]
    SelectorMismatch { expected: String, got: String },

    #[error("[beam-cli] invalid topic count: expected {expected}, got {got}")]
    InvalidTopicCount { expected: usize, got: usize },

    #[error("[beam-cli] transaction failed with status {status}: {tx_hash}")]
    TransactionFailed { status: u64, tx_hash: String },

    #[error("[beam-cli] transaction receipt missing status: {tx_hash}")]
    TransactionStatusMissing { tx_hash: String },

    #[error("[beam-cli] transaction not found: {tx_hash}")]
    TransactionNotFound { tx_hash: String },

    #[error("[beam-cli] block not found: {block}")]
    BlockNotFound { block: String },

    #[error("[beam-cli] invalid function signature: {signature}")]
    InvalidFunctionSignature { signature: String },

    #[error("[beam-cli] invalid abi argument for {kind}: {value}")]
    InvalidAbiArgument { kind: String, value: String },

    #[error("[beam-cli] expected {expected} ABI arguments, got {got}")]
    InvalidArgumentCount { expected: usize, got: usize },

    #[error("[beam-cli] key derivation failed")]
    KeyDerivationFailed,

    #[error("[beam-cli] password cannot be empty or whitespace only")]
    PasswordBlank,

    #[error("[beam-cli] password confirmation does not match")]
    PasswordConfirmationMismatch,

    #[error("[beam-cli] decryption failed")]
    DecryptionFailed,

    #[error(
        "[beam-cli] decrypted wallet key does not match stored address: stored {stored}, derived {derived}"
    )]
    StoredWalletAddressMismatch { derived: String, stored: String },

    #[error("[beam-cli] release asset not found for target {target}")]
    ReleaseAssetNotFound { target: String },

    #[error("[beam-cli] release asset digest missing: {asset}")]
    ReleaseAssetDigestMissing { asset: String },

    #[error("[beam-cli] invalid release asset digest for {asset}: {digest}")]
    InvalidReleaseAssetDigest { asset: String, digest: String },

    #[error(
        "[beam-cli] release asset checksum mismatch for {asset}: expected {expected}, got {actual}"
    )]
    ReleaseAssetChecksumMismatch {
        actual: String,
        asset: String,
        expected: String,
    },

    #[error("[beam-cli] unsupported platform: {os}/{arch}")]
    UnsupportedPlatform { arch: String, os: String },

    #[error("[beam-cli] unknown repl command: {command}")]
    UnknownReplCommand { command: String },

    #[error("[beam-cli] interrupted")]
    Interrupted,

    #[error("[beam-cli] internal error")]
    Internal(#[from] InternalError),
}
