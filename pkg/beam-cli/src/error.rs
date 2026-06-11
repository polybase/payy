// lint-long-file-override allow-max-lines=400
use contextful::{FromContextful, InternalError};

use crate::apps::Error as AppError;

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

    #[error("[beam-cli] privacy is not configured for chain: {chain}")]
    PrivacyNotConfigured { chain: String },

    #[error("[beam-cli] unsupported privacy standard {standard} v{version}")]
    UnsupportedPrivacyStandard { standard: String, version: u32 },

    #[error("[beam-cli] privacy is not available on {chain}: {feature}")]
    PrivacyFeatureUnsupported { chain: String, feature: String },

    #[error("[beam-cli] invalid privacy feature: {feature}")]
    InvalidPrivacyFeature { feature: String },

    #[error(
        "[beam-cli] private mint requires ERC20 approval first: beam erc20 approve {token} {spender} {amount}"
    )]
    PrivacyApprovalRequired {
        amount: String,
        spender: String,
        token: String,
    },

    #[error("[beam-cli] invalid private address: {value}")]
    InvalidPrivateAddress { value: String },

    #[error("[beam-cli] privacy state not found: {id}")]
    PrivacyStateNotFound { id: String },

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

    #[error("[beam-cli] invalid recovery phrase")]
    InvalidRecoveryPhrase,

    #[error("[beam-cli] expected {expected} recovery phrase words, got {got}")]
    InvalidRecoveryPhraseWordCount { expected: usize, got: usize },

    #[error("[beam-cli] recovery phrase entropy must be 32 bytes, got {length}")]
    InvalidRecoveryPhraseEntropyLength { length: usize },

    #[error("[beam-cli] recovery phrase maps to an invalid private key")]
    InvalidRecoveryPhrasePrivateKey,

    #[error("[beam-cli] recovery phrase derives {derived}, not expected wallet address {expected}")]
    RecoveryPhraseAddressMismatch { derived: String, expected: String },

    #[error("[beam-cli] invalid address: {value}")]
    InvalidAddress { value: String },

    #[error("[beam-cli/contract] invalid contract address: {value}")]
    InvalidContractAddress { value: String },

    #[error(
        "[beam-cli/contract] rpc chain mismatch for {chain}: expected {expected}, got {actual}"
    )]
    ContractRpcChainMismatch {
        actual: u64,
        chain: String,
        expected: u64,
    },

    #[error("[beam-cli/contract] rpc lookup failed: {reason}")]
    ContractRpcLookupFailed { reason: String },

    #[error("[beam-cli/contract] no runtime code at {address}")]
    ContractNoRuntimeCode { address: String },

    #[error(
        "[beam-cli/contract] Sourcify artifact not found for {address}: {artifact}{runtime_check}"
    )]
    ContractSourcifyNotVerified {
        address: String,
        artifact: String,
        runtime_check: String,
    },

    #[error(
        "[beam-cli/contract] Sourcify runtime not verified for {address}: {artifact}{runtime_check}"
    )]
    ContractSourcifyRuntimeNotVerified {
        address: String,
        artifact: String,
        runtime_check: String,
    },

    #[error("[beam-cli/contract] Sourcify does not support chain {chain_id}")]
    ContractSourcifyChainUnsupported { chain_id: u64 },

    #[error("[beam-cli/contract] Sourcify lookup failed for {address}: {reason}")]
    ContractSourcifyLookupFailed { address: String, reason: String },

    #[error("[beam-cli/contract] Sourcify response exceeded {cap_bytes} bytes")]
    ContractSourcifyResponseTooLarge { cap_bytes: usize },

    #[error("[beam-cli/contract] malformed Sourcify response: {reason}")]
    ContractSourcifyMalformedResponse { reason: String },

    #[error("[beam-cli/contract] source path not found: {path}")]
    ContractSourcePathNotFound { path: String },

    #[error("[beam-cli/contract] source path is ambiguous: {path}")]
    ContractSourcePathAmbiguous { path: String },

    #[error("[beam-cli/contract] export destination is invalid: {path}")]
    ContractExportDestinationInvalid { path: String },

    #[error("[beam-cli/contract] export destination is not empty: {path}")]
    ContractExportDestinationNotEmpty { path: String },

    #[error("[beam-cli/contract] export filename collision: {path}")]
    ContractExportPathCollision { path: String },

    #[error("[beam-cli/contract] export write failed: {reason}")]
    ContractExportWriteFailed { reason: String },

    #[error("[beam-cli] invalid transaction hash: {value}")]
    InvalidTransactionHash { value: String },

    #[error("[beam-cli] invalid block selector: {value}")]
    InvalidBlockSelector { value: String },

    #[error("[beam-cli] app error")]
    App(#[source] AppError),

    #[error("[beam-cli] invalid rpc url: {value}")]
    InvalidRpcUrl { value: String },

    #[error("[beam-cli] invalid amount: {value}")]
    InvalidAmount { value: String },

    #[error("[beam-cli] unsupported decimals: {decimals} (max {max})")]
    UnsupportedDecimals { decimals: usize, max: usize },

    #[error("[beam-cli] missing input for beam util {command}")]
    MissingUtilInput { command: String },

    #[error("[beam-cli] fetch request failed")]
    FetchRequestFailed,

    #[error("[beam-cli] fetch payment required")]
    FetchPaymentRequired,

    #[error("[beam-cli] fetch payment rejected")]
    FetchPaymentRejected,

    #[error("[beam-cli] invalid fetch payment response")]
    FetchInvalidPaymentResponse,

    #[error("[beam-cli] fetch payment retry cannot override an existing Authorization header")]
    FetchPaymentAuthorizationConflict,

    #[error(
        "[beam-cli] fetch payment challenge must specify a chain unless --chain or --rpc is provided"
    )]
    FetchPaymentChainRequired,

    #[error(
        "[beam-cli] fetch payment chain mismatch: challenge requested {challenge}, but --chain selected {selected}"
    )]
    FetchPaymentChainMismatch { challenge: String, selected: String },

    #[error("[beam-cli] fetch payment chain not allowed: {chain}")]
    FetchPaymentChainNotAllowed { chain: String },

    #[error("[beam-cli] fetch payment exceeds max fee")]
    FetchPaymentExceedsMaxFee,

    #[error("[beam-cli] fetch payment balance too low")]
    FetchPaymentInsufficientBalance,

    #[error(
        "[beam-cli] fetch payment challenges require https; use --dev only for localhost or loopback HTTP fixtures: {url}"
    )]
    FetchPaymentRequiresHttps { url: String },

    #[error("[beam-cli] invalid http method: {value}")]
    FetchInvalidMethod { value: String },

    #[error("[beam-cli] invalid http header: {value}")]
    FetchInvalidHeader { value: String },

    #[error("[beam-cli] payment transaction was not confirmed: {tx_hash}")]
    FetchPaymentUnconfirmed { tx_hash: String },

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

    #[error("[beam-cli] password cannot be whitespace only")]
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

impl From<AppError> for Error {
    fn from(err: AppError) -> Self {
        Self::App(err)
    }
}
