use element::{Base, Element};
use serde::{Deserialize, Serialize};
#[cfg(feature = "ts-rs")]
use ts_rs::TS;

use crate::ProofBytes;

/// Migration input for proving ownership during address migration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Migrate {
    /// The private key of the owner
    pub owner_pk: Element,
    /// The old address (public input)
    pub old_address: Element,
    /// The new address (public input)
    pub new_address: Element,
}

impl Migrate {
    /// Create a new migration input
    #[must_use]
    pub fn new(owner_pk: Element, old_address: Element, new_address: Element) -> Self {
        Self {
            owner_pk,
            old_address,
            new_address,
        }
    }
}

/// Migration public inputs
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-rs", derive(TS))]
#[cfg_attr(feature = "ts-rs", ts(export))]
pub struct MigratePublicInput {
    /// The old address being migrated from
    #[cfg_attr(feature = "ts-rs", ts(as = "String"))]
    pub old_address: Element,
    /// The new address being migrated to
    #[cfg_attr(feature = "ts-rs", ts(as = "String"))]
    pub new_address: Element,
}

impl MigratePublicInput {
    /// Convert the public inputs to field elements
    #[must_use]
    pub fn to_fields(&self) -> Vec<Base> {
        vec![self.old_address.to_base(), self.new_address.to_base()]
    }
}

/// Migration proof output
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "ts-rs", derive(TS))]
#[cfg_attr(feature = "ts-rs", ts(export))]
pub struct MigrateProof {
    /// The proof bytes (without public inputs)
    pub proof: ProofBytes,
    /// The public inputs
    pub public_inputs: MigratePublicInput,
}
