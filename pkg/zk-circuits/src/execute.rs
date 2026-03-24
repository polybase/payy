use acvm::FieldElement;
use acvm::acir::native_types::WitnessStack;
use bn254_blackbox_solver::Bn254BlackBoxSolver;
use nargo::errors::try_to_diagnose_runtime_error;
use nargo::foreign_calls::DefaultForeignCallBuilder;
use noirc_abi::InputMap;
use noirc_abi::input_parser::InputValue;
use noirc_artifacts::debug::DebugArtifact;
use noirc_driver::CompiledProgram;

#[derive(Debug)]
pub struct ExecutionResults {
    #[allow(dead_code)]
    pub actual_return: Option<InputValue>,
    pub witness_stack: WitnessStack<FieldElement>,
}

pub fn execute_program_and_decode(
    program: &CompiledProgram,
    inputs_map: &InputMap,
    pedantic_solving: bool,
) -> Result<ExecutionResults, Box<dyn std::error::Error + Send + Sync>> {
    let witness_stack = execute_program(program, inputs_map, pedantic_solving)?;

    // Get the entry point witness for the ABI
    let main_witness = &witness_stack
        .peek()
        .expect("Should have at least one witness on the stack")
        .witness;
    let (_, actual_return) = program.abi.decode(main_witness)?;

    Ok(ExecutionResults {
        actual_return,
        witness_stack,
    })
}

pub fn execute_program(
    compiled_program: &CompiledProgram,
    inputs_map: &InputMap,
    pedantic_solving: bool,
) -> Result<WitnessStack<FieldElement>, Box<dyn std::error::Error + Send + Sync>> {
    let initial_witness = compiled_program.abi.encode(inputs_map, None)?;

    let solved_witness_stack_err = nargo::ops::execute_program(
        &compiled_program.program,
        initial_witness,
        &Bn254BlackBoxSolver(pedantic_solving),
        &mut DefaultForeignCallBuilder {
            output: std::io::stdout(),
            enable_mocks: false,
            // resolver_url: foreign_call_resolver_url.map(|s| s.to_string()),
            // root_path,
            // package_name,
        }
        .build(),
    );
    match solved_witness_stack_err {
        Ok(solved_witness_stack) => Ok(solved_witness_stack),
        Err(err) => {
            let debug_artifact = DebugArtifact {
                debug_symbols: compiled_program.debug.clone(),
                file_map: compiled_program.file_map.clone(),
            };

            if let Some(diagnostic) =
                try_to_diagnose_runtime_error(&err, &compiled_program.abi, &compiled_program.debug)
            {
                diagnostic.report(&debug_artifact, false);
            }

            Err(Box::new(err))
        }
    }
}
