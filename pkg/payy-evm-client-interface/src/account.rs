use std::sync::Arc;

pub use crate::address::{PrivacyAddress, PrivacyAddressPrefix};

use crate::evm::{Address, PayyEvmSubmitter};
use crate::privacy::PrivacySigner;

/// Privacy account selector.
#[derive(Clone)]
pub enum PrivacyAccount {
    /// Resolve this address through the configured client privacy signer.
    Address(PrivacyAddress),
    /// Use the supplied signer for this account.
    Signer(PrivacySignerAccount),
}

impl std::fmt::Debug for PrivacyAccount {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Address(address) => fmt.debug_tuple("Address").field(address).finish(),
            Self::Signer(account) => fmt.debug_tuple("Signer").field(account).finish(),
        }
    }
}

impl PrivacyAccount {
    /// Return the selected privacy address.
    #[must_use]
    pub const fn privacy_address(&self) -> PrivacyAddress {
        match self {
            Self::Address(address) => *address,
            Self::Signer(account) => account.privacy_address,
        }
    }
}

impl From<PrivacyAddress> for PrivacyAccount {
    fn from(address: PrivacyAddress) -> Self {
        Self::Address(address)
    }
}

/// Signer-backed privacy account selector.
#[derive(Clone)]
pub struct PrivacySignerAccount {
    /// Selected privacy address.
    pub privacy_address: PrivacyAddress,
    /// Signer for this address.
    pub signer: Arc<dyn PrivacySigner>,
}

impl std::fmt::Debug for PrivacySignerAccount {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt.debug_struct("PrivacySignerAccount")
            .field("privacy_address", &self.privacy_address)
            .finish_non_exhaustive()
    }
}

/// EVM account selector.
#[derive(Clone)]
pub enum EvmAccount {
    /// Resolve this address through the configured submitter.
    Address(Address),
    /// Use the supplied submitter for this account.
    Signer(EvmSignerAccount),
}

impl std::fmt::Debug for EvmAccount {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Address(address) => fmt.debug_tuple("Address").field(address).finish(),
            Self::Signer(account) => fmt.debug_tuple("Signer").field(account).finish(),
        }
    }
}

impl EvmAccount {
    /// Return the selected EVM address.
    #[must_use]
    pub const fn address(&self) -> Address {
        match self {
            Self::Address(address) => *address,
            Self::Signer(account) => account.address,
        }
    }
}

impl From<Address> for EvmAccount {
    fn from(address: Address) -> Self {
        Self::Address(address)
    }
}

/// Signer-backed EVM account selector.
#[derive(Clone)]
pub struct EvmSignerAccount {
    /// Selected EVM address.
    pub address: Address,
    /// Submitter for this address.
    pub submitter: Arc<dyn PayyEvmSubmitter>,
}

impl std::fmt::Debug for EvmSignerAccount {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        fmt.debug_struct("EvmSignerAccount")
            .field("address", &self.address)
            .finish_non_exhaustive()
    }
}
