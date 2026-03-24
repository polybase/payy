//! Shared data models and Diesel-backed data access helpers.

pub mod diagnostics;
pub mod faucet;
pub mod ip_data;
pub mod migrate_elements;
pub mod nft;
pub mod payment;
pub mod registry_note;
pub mod support;
pub mod support_canned_response;
pub mod support_tag;
pub mod udh;
pub mod wallet;
pub mod wallet_activity;
pub mod wallet_auth;
pub mod wallet_backup;
pub mod wallet_notes;

#[cfg(feature = "diesel")]
pub(crate) use diesel_util::derive_pg_text_enum;
