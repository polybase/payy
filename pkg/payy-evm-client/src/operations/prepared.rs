use std::sync::Arc;

use payy_evm_client_interface::{
    PayyEvmSubmitter, PreparedOperationResult, PreparedPrivacyCall, PreparedPrivacyCallSource,
};

use crate::client::PrivacyNamespace;

/// Prepared operation with submission capability.
#[derive(Clone)]
pub struct Prepared<TPayload> {
    pub(super) result: PreparedOperationResult<TPayload>,
    pub(super) client: PrivacyNamespace,
    pub(super) submitter: Option<Arc<dyn PayyEvmSubmitter>>,
}

impl<TPayload> Prepared<TPayload> {
    /// Return the prepared operation result.
    #[must_use]
    pub const fn result(&self) -> &PreparedOperationResult<TPayload> {
        &self.result
    }

    /// Return the prepared bridge call.
    #[must_use]
    pub const fn prepared_call(&self) -> &PreparedPrivacyCall {
        &self.result.prepared_call
    }

    /// Return the operation-specific payload.
    #[must_use]
    pub const fn payload(&self) -> &TPayload {
        &self.result.payload
    }

    /// Consume the wrapper into the prepared operation result.
    #[must_use]
    pub fn into_result(self) -> PreparedOperationResult<TPayload> {
        self.result
    }
}

impl<TPayload> PreparedPrivacyCallSource for Prepared<TPayload> {
    fn prepared_privacy_call(&self) -> &PreparedPrivacyCall {
        &self.result.prepared_call
    }
}
