// lint-long-file-override allow-max-lines=300
use contextful::{FromContextful, InternalError};
use element::Element;
use kyc::{KycStatus, KycUpdateRequired};
use network::Network;
use rpc::{
    HTTPErrorConversion,
    code::ErrorCode,
    error::{ErrorOutput, HTTPError, TryFromHTTPError},
};
use thiserror::Error;

pub use crate::account_kind::AccountKind;
pub use crate::result::Result;
use crate::transaction::{FundingKind, FundingStatus};

#[derive(Debug, Error, HTTPErrorConversion, FromContextful)]
pub enum Error {
    #[error("[ramps-interface] unable to get provider client")]
    UnableToGetProviderClient,

    #[error("[ramps-interface] failed to settle transaction")]
    FailedToSettledTxn,

    #[error("[ramps-interface] wallet registration failed")]
    WalletRegistrationFailed,

    #[error("[ramps-interface] card is frozen")]
    #[permission_denied("card-is-frozen")]
    CardFrozen,

    #[error("[ramps-interface] fraud block")]
    #[permission_denied("fraud-block")]
    FraudBlock,

    #[error("[ramps-interface] payy owner insufficient funds")]
    PayyOwnerInsufficientFunds,

    #[error("[ramps-interface] invalid encryption key")]
    #[bad_request("missing-encryption-key")]
    InvalidEncryptionKey,

    #[error("[ramps-interface] invalid decimal string")]
    #[bad_request("invalid-decimal")]
    InvalidDecimalString,

    #[error("[ramps-interface] invalid phone format")]
    InvalidPhoneFormat,

    #[error("[ramps-interface] invalid amount")]
    #[bad_request("invalid-amount")]
    InvalidAmount {
        min: Option<Element>,
        max: Option<Element>,
    },

    #[error("[ramps-interface] transaction amount does not match funding note amount")]
    #[failed_precondition("transaction-amount-mismatch")]
    AmountMismatch { expected: Element, got: Element },

    #[error("[ramps-interface] invalid swap amount")]
    #[bad_request("invalid-swap-amount")]
    InvalidSwapAmount,

    #[error("[ramps-interface] invalid provider")]
    #[bad_request("invalid-provider")]
    InvalidProvider,

    #[error("[ramps-interface] funding status is invalid: {0:?}")]
    InvalidFundingStatus(Option<FundingStatus>),

    #[error("[ramps-interface] external record not found")]
    ExternalRecordNotFound,

    #[error("[ramps-interface] exceeded provider limit")]
    #[bad_request("exceeded-provider-limit")]
    ExceededProviderLimit,

    #[error("[ramps-interface] kyc required")]
    #[failed_precondition("kyc-required")]
    KycRequired(KycStatus),

    #[error("[ramps-interface] kyc update required")]
    KycUpdateRequired(KycUpdateRequired),

    #[error("[ramps-interface] missing required kyc docs")]
    MissingRequiredKyCDocs,

    #[error("[ramps-interface] missing kyc data")]
    #[bad_request("missing-kyc")]
    MissingKyc,

    #[error("[ramps-interface] missing kyc required field")]
    #[bad_request("missing-kyc-field")]
    MissingKycField(String),

    #[error("[ramps-interface] invalid document id format")]
    #[bad_request("invalid-document-id")]
    InvalidDocumentId,

    #[error("[ramps-interface] document not found")]
    #[not_found("document-not-found")]
    DocumentNotFound,

    #[error("[ramps-interface] unsupported currency")]
    #[bad_request("unsupported-currency")]
    UnsupportedCurrency,

    #[error("[ramps-interface] unsupported network")]
    UnsupportedNetwork,

    #[error(
        "[ramps-interface] invalid transaction kind state for from_network {from_network:?}, to_network {to_network:?}, funding_kind {funding_kind:?}"
    )]
    InvalidTransactionKindState {
        from_network: Network,
        to_network: Network,
        funding_kind: FundingKind,
    },

    #[error("[ramps-interface] unsupported swap pair")]
    #[bad_request("unsupported-swap-pair")]
    UnsupportedSwapPair,

    #[error("[ramps-interface] transaction note kind does not match funding note kind")]
    #[failed_precondition("transaction-note-kind-mismatch")]
    NoteKindMismatch { expected: Element, got: Element },

    #[error("[ramps-interface] unsupported currency for provider")]
    UnsupportedProviderCurrency,

    #[error("[ramps-interface] unsupported country for provider")]
    #[bad_request("unsupported-provider-country")]
    UnsupportedProviderCountry,

    #[error("[ramps-interface] unsupported network for provider")]
    #[bad_request("unsupported-provider-network")]
    UnsupportedProviderNetwork,

    #[error("[ramps-interface] missing country")]
    #[bad_request("missing-country")]
    MissingCountry,

    #[error("[ramps-interface] method not found")]
    #[not_found("method-not-found")]
    MethodNotFound,

    #[error("[ramps-interface] quote missing both to/from amount")]
    QuoteMissingBothAmounts,

    #[error("[ramps-interface] invalid quote")]
    #[bad_request("invalid-quote")]
    InvalidQuote,

    #[error("[ramps-interface] missing quote")]
    MissingQuote,

    #[error("[ramps-interface] quote not found")]
    #[not_found("quote-not-found")]
    QuoteNotFound,

    #[error("[ramps-interface] quote provider mismatch")]
    #[bad_request("quote-provider-mismatch")]
    QuoteProviderMismatch,

    #[error("[ramps-interface] payy network is required in either to or from")]
    #[bad_request("payy-network-required")]
    PayyNetworkRequired,

    #[error("[ramps-interface] payy ramps account is missing")]
    #[internal("payy-account-missing")]
    PayyAccountMissing,

    #[error("[ramps-interface] account not found")]
    #[not_found("account-not-found")]
    AccountNotFound { kind: AccountKind, id: String },

    #[error("[ramps-interface] account already exists")]
    #[already_exists("account-already-exists")]
    AccountAlreadyExists,

    #[error("[ramps-interface] method required")]
    #[bad_request("method-required")]
    MethodRequired,

    #[error("[ramps-interface] missing evm address")]
    #[failed_precondition("evm-address-required")]
    MissingEvmAddress,

    #[error("[ramps-interface] missing external id")]
    #[internal("missing-external-id")]
    MissingExternalId,

    #[error("[ramps-interface] invalid method details")]
    #[bad_request("invalid-method-details")]
    InvalidMethodDetails,

    #[error("[ramps-interface] account creation requires kyc")]
    AccountCreationRequiresKyc,

    #[error("[ramps-interface] account provider mismatch")]
    #[bad_request("account-provider-mismatch")]
    AccountProviderMismatch,

    #[error("[ramps-interface] account address mismatch")]
    #[bad_request("account-address-mismatch")]
    AccountAddressMismatch,

    #[error("[ramps-interface] missing required network identifier field")]
    #[bad_request("missing-required-network-identifier-field")]
    MissingRequiredNetworkIdentifierField(String),

    #[error("[ramps-interface] insufficient funds")]
    #[failed_precondition("insufficient-funds")]
    InsufficientFunds,

    #[error("[ramps-interface] token price not found")]
    #[failed_precondition("token-price-unavailable")]
    TokenPriceNotFound,

    #[error("[ramps-interface] token price is stale")]
    #[failed_precondition("token-price-stale")]
    TokenPriceStale,

    #[error("[ramps-interface] transaction can only be cancelled")]
    #[bad_request("transaction-can-only-be-cancelled")]
    TransactionCanOnlyBeCancelled,

    #[error("[ramps-interface] transaction not found")]
    #[not_found("transaction-not-found")]
    TransactionNotFound,

    #[error("[ramps-interface] transaction is in progress and cannot be cancelled")]
    #[failed_precondition("transaction-in-progress-cannot-be-cancelled")]
    TransactionCannotBeCancelled,

    #[error("[ramps-interface] transaction cannot be funded in its current state")]
    #[failed_precondition("transaction-cannot-be-funded")]
    TransactionCannotBeFunded,

    #[error("[ramps-interface] transaction evm address cannot be updated")]
    #[failed_precondition("transaction-evm-address-cannot-be-updated")]
    TransactionEvmAddressCannotBeUpdated,

    #[error("[ramps-interface] transaction local id cannot be updated")]
    #[failed_precondition("transaction-local-id-cannot-be-updated")]
    TransactionLocalIdCannotBeUpdated,

    #[error("[ramps-interface] invalid currency code")]
    InvalidCurrencyCode,

    #[error("[ramps-interface] declined transaction with spent notes")]
    DeclinedTransactionWithSpentNotes,

    #[error("[ramps-interface] funding note is already spent")]
    #[failed_precondition("note-already-spent")]
    NoteAlreadySpent,

    #[error("[ramps-interface] funding note was not confirmed")]
    #[failed_precondition("funding-note-not-confirmed")]
    FundingNoteNotConfirmed,

    #[error(
        "[ramps-interface] MCC 6012 transactions are blocked unless they are $0 Visa Provisioning Service transactions"
    )]
    #[bad_request("declined-mcc-6012-transaction")]
    DeclinedMcc6012Transaction,

    #[error("[ramps-interface] transaction blocked due to restricted MCC: {0}")]
    #[bad_request("declined-mcc-transaction")]
    DeclinedMccTransaction(u16),

    #[error("[ramps-interface] daily spending limit exceeded")]
    #[bad_request("daily-spending-limit-exceeded")]
    DailySpendingLimitExceeded,

    #[error("[ramps-interface] PAYY transactions are blocked to prevent circular usage")]
    #[bad_request("declined-payy-transaction")]
    DeclinedPayyTransaction,

    #[error("[ramps-interface] transaction below minimum amount of $0.10")]
    #[bad_request("minimum-transaction-amount")]
    MinimumTransactionAmount,

    #[error("[ramps-interface] invalid auth")]
    #[unauthenticated("unauthorized-to-perform-action")]
    InvalidAuth,

    #[error("[ramps-interface] invalid admin key")]
    #[unauthenticated("invalid-admin-key")]
    InvalidAdminKey,

    #[error("[ramps-interface] permission denied: admin token lacks required scope")]
    #[permission_denied("permission-denied")]
    AdminTokenLacksRequiredScope,

    /// Internal error.
    #[error("[ramps-interface] internal error")]
    Internal(#[from] InternalError),
}
