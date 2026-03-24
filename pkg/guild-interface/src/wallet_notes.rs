pub use data::wallet_notes::{Status, WalletNote};
use rpc::{
    code::ErrorCode,
    error::{ErrorOutput, HTTPError, TryFromHTTPError},
};
use rpc_error_convert::HTTPErrorConversion;
use serde::{Deserialize, Serialize};

/// Data for address mismatch error
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct AddressMismatch {
    /// Address of authenticated user
    pub authenticated_address: String,
    /// Address in the activity body
    pub note_adress: Vec<String>,
}

/// RPC errors for activity
#[derive(Debug, Clone, thiserror::Error, HTTPErrorConversion, Serialize, Deserialize)]
pub enum Error {
    /// Invalid signature length, epected uncompressed signature
    #[bad_request("wallet-notes-address-mismatch")]
    #[error("authenticated address does not match activity address")]
    AuthAddressMismatch(AddressMismatch),
}

/// Request body for upserting notes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletNoteRequest {
    /// A set of notes to update in the database
    pub notes: Vec<WalletNote>,
}
