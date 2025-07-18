use crate::{backend::Backend, execute::execute_program_and_decode, Result};
use noirc_abi::InputMap;
use noirc_driver::CompiledProgram;

pub fn prove<B: Backend>(
    compiled_program: &CompiledProgram,
    program: &[u8],
    bytecode: &[u8],
    inputs_map: &InputMap,
    recursive: bool,
    oracle_hash_keccak: bool,
) -> Result<Vec<u8>> {
    let results = execute_program_and_decode(compiled_program, inputs_map, false)?;

    let witness = bincode::serialize(&results.witness_stack)?;

    B::prove(program, bytecode, &witness, recursive, oracle_hash_keccak)
    // Ok(proof)
}

// pub fn prove_witness(
//     bb_path: &PathBuf,
//     program_path: &PathBuf,
//     witness: &[u8],
//     recursive: bool,
// ) -> Result<Vec<u8>> {
//     let mut witness_file = NamedTempFile::new()?;
//     witness_file.write_all(witness)?;
//     witness_file.flush()?;

//     // Create a temporary directory for the output
//     let temp_dir = tempfile::tempdir()?;
//     let output_dir_path = temp_dir.path();

//     let mut cmd = Command::new(bb_path);
//     cmd.arg("prove")
//         .arg("--scheme")
//         .arg("ultra_honk")
//         .arg("-b")
//         .arg(program_path)
//         .arg("-w")
//         .arg(witness_file.path())
//         .arg("-o")
//         .arg(output_dir_path);

//     if recursive {
//         cmd.arg("--recursive");
//     }

//     let output = cmd.output()?;
//     if !output.status.success() {
//         let stderr = String::from_utf8(output.stderr)?;
//         return Err(stderr.into());
//     }

//     let proof_path = output_dir_path.join("proof");
//     let proof_data = std::fs::read(proof_path)?;

//     println!("{:?}", proof_data[0..4].to_vec());

//     Ok(proof_data[4..].to_vec())
// }
