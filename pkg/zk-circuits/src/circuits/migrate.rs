use zk_primitives::{Migrate, MigrateProof, MigratePublicInput};

use crate::circuits::Proof;

use super::conversions::impl_circuit_proof_conversions;
use super::generated::migrate::{MigrateInput, MigratePublicInputs as CircuitMigratePublicInputs};

impl From<Migrate> for MigrateInput {
    fn from(migrate: Migrate) -> Self {
        let Migrate {
            owner_pk,
            old_address,
            new_address,
        } = migrate;

        MigrateInput {
            owner_pk,
            old_address,
            new_address,
        }
    }
}

impl_circuit_proof_conversions!(
    MigratePublicInput,
    CircuitMigratePublicInputs,
    MigrateProof,
    Proof<CircuitMigratePublicInputs>,
    [old_address, new_address]
);
