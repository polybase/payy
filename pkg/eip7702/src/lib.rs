//! EIP-7702 support library split into focused modules.
//! Modules: auth, eip712, txn, relayer, types.

mod auth;
mod eip712;
mod error;
mod relayer;
mod txn;
mod types;

pub use crate::auth::{auth_message_hash, recover_authority, sign_authorization};
pub use crate::eip712::{
    eip712_domain_separator, encode_execute_meta_many_call, execute_meta_many_digest,
};
pub use crate::error::{Error, Result};
pub use crate::relayer::HttpEip7702Relayer;
pub use crate::txn::{build_set_code_tx_json, delegation_indicator_code};
pub use crate::types::{Authorization, Eip7702Relayer, Eip7702RelayerMock, MetaCall};
