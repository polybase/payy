use std::io::Read;

use acvm::AcirField;
use async_trait::async_trait;
use base64::Engine;
use element::Base;
use flate2::read::GzDecoder;

mod agg_agg;
mod agg_final;
mod agg_utxo;
mod conversions;
mod migrate;
pub mod note_v2;
mod notes;
mod points;
mod proc_macro_interface;
mod proof;
mod signature;
#[cfg(test)]
mod tests;
mod utxo;

pub use proof::Proof;
pub use utxo::*;

use crate::{
    Prove, Result,
    circuits::proc_macro_interface::{ProofInputs, PublicInputs},
    prove::prove,
};

#[async_trait]
impl<T: ProofInputs> Prove for T
where
    T::PublicInputs: Send + Sync,
    T: Send + Sync,
{
    type Proof = Proof<T::PublicInputs>;
    async fn prove(
        &self,
        bb_backend: &dyn barretenberg_interface::BbBackend,
    ) -> Result<Self::Proof> {
        let inputs = self.input_map();
        let proof_bytes = prove(
            bb_backend,
            self.compiled_program(),
            Self::PROGRAM.as_bytes(),
            self.bytecode(),
            Self::KEY,
            &inputs,
            T::PublicInputs::ORACLE_HASH_KECCAK,
        )
        .await?;
        Ok(Proof::from_raw_proof_bytes(proof_bytes))
    }
}

macro_rules! generate_inputs {
    ($fixture_path:literal, $mod:ident) => {
        generate_inputs!($fixture_path, $mod, false);
    };
    ($fixture_path:literal, $mod:ident, $oracle_keccak:literal) => {
        noir_abi_inputs_macro::noir_abi_inputs!($fixture_path, $mod, $oracle_keccak, crate::circuits::proc_macro_interface);

        paste::paste!(
            pub struct [<$mod:camel Circuit>];

            #[async_trait::async_trait]
            impl $crate::Circuit<$mod::[<$mod:camel Input>], super::Proof<$mod::[<$mod:camel PublicInputs>]>> for [<$mod:camel Circuit>] {
                async fn prove(
                    &self,
                    input: &$mod::[<$mod:camel Input>],
                    bb_backend: std::sync::Arc<dyn barretenberg_interface::BbBackend>,
                ) -> $crate::Result<super::Proof<$mod::[<$mod:camel PublicInputs>]>> {
                    let _ = self;
                    <$mod::[<$mod:camel Input>] as $crate::Prove>::prove(input, bb_backend.as_ref()).await
                }
            }

            pub trait [<$mod:camel CircuitInterface>]: $crate::Circuit<$mod::[<$mod:camel Input>], super::Proof<$mod::[<$mod:camel PublicInputs>]>> {}

            impl<T> [<$mod:camel CircuitInterface>] for T
            where
                T: $crate::Circuit<$mod::[<$mod:camel Input>], super::Proof<$mod::[<$mod:camel PublicInputs>]>>
            {}
        );
    };
}

// CI treats warnings as errors; remove to surface duplicate-struct warnings.
#[allow(deprecated)]
pub mod generated {
    noir_abi_inputs_macro::noir_abi_shared_structs!(
        "../../fixtures/circuits",
        crate::circuits::proc_macro_interface
    );
    generate_inputs!("../../fixtures/circuits/agg_agg", agg_agg);
    generate_inputs!("../../fixtures/circuits/agg_final", agg_final, true);
    generate_inputs!("../../fixtures/circuits/agg_utxo", agg_utxo);
    generate_inputs!("../../fixtures/circuits/migrate", migrate);
    generate_inputs!("../../fixtures/circuits/points", points);
    generate_inputs!("../../fixtures/circuits/signature", signature);
    generate_inputs!("../../fixtures/circuits/utxo", utxo);
    generate_inputs!("../../fixtures/circuits/erc20_transfer", erc20_transfer);
    generate_inputs!("../../fixtures/circuits/transfer", transfer);
    generate_inputs!("../../fixtures/circuits/mint", mint);
    generate_inputs!("../../fixtures/circuits/burn", burn);
}

pub fn get_bytecode_from_program(program_json: &str) -> Vec<u8> {
    let mut program = serde_json::from_str::<serde_json::Value>(program_json).unwrap();
    let bytecode_base64 = program.get_mut("bytecode").unwrap().as_str().unwrap();
    let bytecode_gzipped = base64::engine::general_purpose::STANDARD
        .decode(bytecode_base64)
        .unwrap();
    let mut decoder = GzDecoder::new(&bytecode_gzipped[..]);
    let mut bytecode = Vec::new();
    decoder.read_to_end(&mut bytecode).unwrap();

    bytecode
}

pub fn parse_key_fields(key_fields_json: &[u8]) -> Vec<Base> {
    let fields = serde_json::from_slice::<Vec<String>>(key_fields_json).unwrap();

    fields
        .into_iter()
        .map(|field| Base::from_hex(&field).unwrap())
        .collect()
}
