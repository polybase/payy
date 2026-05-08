use alloy::{
    primitives::{Address as AlloyAddress, Bytes as AlloyBytes, TxKind, U256},
    rpc::types::{TransactionInput, TransactionRequest},
};
use payy_evm_client_interface::{
    Address, Error, PayyEvmTransactionRequest, PreparedPrivacyCallSource, Result,
    ValidationErrorKind,
};

/// Options for converting a Payy prepared call into an Alloy transaction request.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct AlloyTransactionOptions {
    /// Optional chain ID to validate against the prepared call.
    pub chain_id: Option<u64>,
    /// Optional sender to validate against the prepared bridge request.
    pub from: Option<Address>,
}

/// Convert any prepared Payy privacy call source into an Alloy transaction request.
pub fn to_alloy_transaction<T>(prepared: &T) -> Result<TransactionRequest>
where
    T: PreparedPrivacyCallSource + ?Sized,
{
    to_alloy_transaction_with_options(prepared, AlloyTransactionOptions::default())
}

/// Convert any prepared Payy privacy call source into an Alloy transaction request.
pub fn to_alloy_transaction_with_options<T>(
    prepared: &T,
    options: AlloyTransactionOptions,
) -> Result<TransactionRequest>
where
    T: PreparedPrivacyCallSource + ?Sized,
{
    let prepared_call = prepared.prepared_privacy_call();
    let chain_id = match options.chain_id {
        Some(chain_id) if chain_id != prepared_call.chain_id => {
            return Err(Error::ChainIdMismatch {
                expected: prepared_call.chain_id,
                actual: chain_id,
            });
        }
        Some(chain_id) => chain_id,
        None => prepared_call.chain_id,
    };

    payy_to_alloy_transaction_request(
        &prepared_call.bridge_request,
        AlloyTransactionOptions {
            chain_id: Some(chain_id),
            from: options.from,
        },
    )
}

pub(crate) fn payy_to_alloy_transaction_request(
    request: &PayyEvmTransactionRequest,
    options: AlloyTransactionOptions,
) -> Result<TransactionRequest> {
    let from = match (request.from, options.from) {
        (Some(request_from), Some(options_from)) if request_from != options_from => {
            return Err(Error::Validation {
                kind: ValidationErrorKind::EvmAccountMismatch,
            });
        }
        (Some(request_from), _) => Some(request_from),
        (None, options_from) => options_from,
    };

    Ok(TransactionRequest {
        from: from.map(AlloyAddress::from),
        to: Some(TxKind::Call(AlloyAddress::from(request.to))),
        gas: request.gas_limit,
        value: Some(U256::from(request.value)),
        input: TransactionInput::new(AlloyBytes::from(request.data.clone())),
        chain_id: options.chain_id,
        ..TransactionRequest::default()
    })
}
