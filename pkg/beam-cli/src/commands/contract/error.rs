use crate::error::Error as BeamError;

pub(super) type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Clone, Debug)]
pub(super) struct RuntimeUnchecked {
    pub(super) reason: Option<String>,
}

#[derive(Debug, thiserror::Error)]
pub(super) enum Error {
    #[error("invalid_contract_address")]
    InvalidContractAddress { value: String },

    #[error("rpc_chain_mismatch")]
    RpcChainMismatch {
        actual: u64,
        chain: String,
        expected: u64,
    },

    #[error("rpc_lookup_failed")]
    RpcLookupFailed { reason: String },

    #[error("no_runtime_code")]
    NoRuntimeCode { address: String },

    #[error("sourcify_not_verified")]
    SourcifyNotVerified {
        address: String,
        artifact: String,
        runtime_unchecked: Option<RuntimeUnchecked>,
    },

    #[error("sourcify_runtime_not_verified")]
    SourcifyRuntimeNotVerified {
        address: String,
        artifact: String,
        runtime_unchecked: Option<RuntimeUnchecked>,
    },

    #[error("sourcify_chain_unsupported")]
    SourcifyChainUnsupported { chain_id: u64 },

    #[error("sourcify_lookup_failed")]
    SourcifyLookupFailed { address: String, reason: String },

    #[error("sourcify_response_too_large")]
    SourcifyResponseTooLarge { cap_bytes: usize },

    #[error("sourcify_malformed_response")]
    SourcifyMalformedResponse { reason: String },

    #[error("source_path_not_found")]
    SourcePathNotFound { path: String },

    #[error("source_path_ambiguous")]
    SourcePathAmbiguous { matches: Vec<String>, path: String },

    #[error("export_destination_invalid")]
    ExportDestinationInvalid { path: String },

    #[error("export_destination_not_empty")]
    ExportDestinationNotEmpty { path: String },

    #[error("export_path_collision")]
    ExportPathCollision { path: String },

    #[error("export_write_failed")]
    ExportWriteFailed { reason: String },
}

impl From<Error> for BeamError {
    fn from(err: Error) -> Self {
        match err {
            Error::InvalidContractAddress { value } => Self::InvalidContractAddress { value },
            Error::RpcChainMismatch {
                actual,
                chain,
                expected,
            } => Self::ContractRpcChainMismatch {
                actual,
                chain,
                expected,
            },
            Error::RpcLookupFailed { reason } => Self::ContractRpcLookupFailed { reason },
            Error::NoRuntimeCode { address } => Self::ContractNoRuntimeCode { address },
            Error::SourcifyNotVerified {
                address,
                artifact,
                runtime_unchecked,
            } => Self::ContractSourcifyNotVerified {
                address,
                artifact,
                runtime_check: runtime_check_message(runtime_unchecked),
            },
            Error::SourcifyRuntimeNotVerified {
                address,
                artifact,
                runtime_unchecked,
            } => Self::ContractSourcifyRuntimeNotVerified {
                address,
                artifact,
                runtime_check: runtime_check_message(runtime_unchecked),
            },
            Error::SourcifyChainUnsupported { chain_id } => {
                Self::ContractSourcifyChainUnsupported { chain_id }
            }
            Error::SourcifyLookupFailed { address, reason } => {
                Self::ContractSourcifyLookupFailed { address, reason }
            }
            Error::SourcifyResponseTooLarge { cap_bytes } => {
                Self::ContractSourcifyResponseTooLarge { cap_bytes }
            }
            Error::SourcifyMalformedResponse { reason } => {
                Self::ContractSourcifyMalformedResponse { reason }
            }
            Error::SourcePathNotFound { path } => Self::ContractSourcePathNotFound { path },
            Error::SourcePathAmbiguous { path, .. } => Self::ContractSourcePathAmbiguous { path },
            Error::ExportDestinationInvalid { path } => {
                Self::ContractExportDestinationInvalid { path }
            }
            Error::ExportDestinationNotEmpty { path } => {
                Self::ContractExportDestinationNotEmpty { path }
            }
            Error::ExportPathCollision { path } => Self::ContractExportPathCollision { path },
            Error::ExportWriteFailed { reason } => Self::ContractExportWriteFailed { reason },
        }
    }
}

fn runtime_check_message(unchecked: Option<RuntimeUnchecked>) -> String {
    match unchecked {
        Some(RuntimeUnchecked {
            reason: Some(reason),
        }) => format!("; runtime code was not checked; RPC check failed: {reason}"),
        Some(RuntimeUnchecked { reason: None }) => "; runtime code was not checked".to_owned(),
        None => String::new(),
    }
}
