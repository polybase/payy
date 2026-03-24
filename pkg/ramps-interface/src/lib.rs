#![warn(clippy::pedantic)]
#![allow(missing_docs)]

//! Core interface definitions, shared data structures, and error types for
//! ramps operations. This crate owns the canonical API contract between the
//! HTTP layer (`ramps-rpc`) and business logic implementations.

pub mod account;
pub mod admin;
pub mod document;
pub mod error;
pub mod event;
pub mod method;
pub mod provider;
pub mod quote;
pub mod transaction;
pub mod util;
pub mod webhooks;

#[cfg(test)]
mod tests;

pub use account::*;
pub use admin::*;
pub use document::*;
pub use error::{Error, Result};
pub use event::*;
pub use method::*;
pub use provider::*;
pub use quote::*;
pub use transaction::*;
pub use util::*;
pub use webhooks::*;

use async_trait::async_trait;
use std::sync::Arc;
use test_spy::spy_mock;

#[cfg(feature = "diesel")]
pub(crate) use diesel_util::derive_pg_text_enum;

/// Root interface that exposes accessors to all ramps sub interfaces.
#[spy_mock]
#[async_trait]
pub trait RampsInterface: Send + Sync {
    /// Accounts domain API.
    fn accounts(&self) -> Arc<dyn account::AccountsInterface>;

    /// Methods domain API.
    fn methods(&self) -> Arc<dyn method::MethodsInterface>;

    /// Transactions domain API.
    fn transactions(&self) -> Arc<dyn transaction::TransactionsInterface>;

    /// Quotes domain API.
    fn quotes(&self) -> Arc<dyn quote::QuotesInterface>;

    /// Admin domain API.
    fn admin(&self) -> Arc<dyn admin::AdminInterface>;

    /// Webhooks domain API.
    fn webhooks(&self) -> Arc<dyn webhooks::WebhooksInterface>;

    /// Documents domain API.
    fn documents(&self) -> Arc<dyn document::DocumentsInterface>;
}
