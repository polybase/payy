use crate::{BbBackend, Error, Result, execute::execute_program_and_decode};
use noirc_abi::InputMap;
use noirc_driver::CompiledProgram;

pub async fn prove(
    bb_backend: &dyn BbBackend,
    compiled_program: &CompiledProgram,
    program: &[u8],
    bytecode: &[u8],
    key: &[u8],
    inputs_map: &InputMap,
    oracle_hash_keccak: bool,
) -> Result<Vec<u8>> {
    let results = execute_program_and_decode(compiled_program, inputs_map, false)
        .map_err(Error::ImplementationSpecific)?;

    let witness = bincode::serde::encode_to_vec(&results.witness_stack, bincode::config::legacy())
        .map_err(|e| Error::ImplementationSpecific(e.into()))?;

    bb_backend
        .prove(program, bytecode, key, &witness, oracle_hash_keccak)
        .await
}
