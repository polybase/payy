use std::{path::PathBuf, thread};

use contextful::ResultContextExt;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::runtime::Handle;
use wasmi::{Engine, Linker, Module, Store};

use crate::{
    apps::{
        Error, Result,
        host::HostMetadata,
        model::{ActionPlan, AppPermissions},
    },
    output::{CommandOutput, OutputMode},
    runtime::BeamApp,
};

use super::{
    HOST_API_VERSION, WASM_FUEL,
    debug::app_debug,
    guest::{HostState, guest_alloc, register_host_imports, typed_func, unpack_ptr_len},
};

const MAX_GUEST_RESPONSE_BYTES: usize = 2 * 1024 * 1024;
// wasmi uses native stack while interpreting long guest calls; run app execution
// off the main thread with enough headroom for JSON-heavy app flows.
const WASM_THREAD_STACK_BYTES: usize = 512 * 1024 * 1024;

pub(super) struct CommandRun {
    pub(super) app: BeamApp,
    pub(super) args: Vec<String>,
    pub(super) engine: Engine,
    pub(super) entrypoint: String,
    pub(super) manifest_id: String,
    pub(super) metadata: HostMetadata,
    pub(super) module_path: PathBuf,
    pub(super) output_mode: OutputMode,
    pub(super) permissions: AppPermissions,
    pub(super) runtime_handle: Handle,
}

#[derive(Debug)]
pub enum GuestCommandResult {
    ActionPlan(ActionPlan),
    Output(CommandOutput),
}

pub(super) fn run_guest_command(input: CommandRun) -> Result<GuestCommandResult> {
    let thread_name = format!("beam-app-wasm-{}", input.manifest_id);
    let thread = thread::Builder::new()
        .name(thread_name)
        .stack_size(WASM_THREAD_STACK_BYTES)
        .spawn(|| run_guest_command_on_thread(input))
        .context("spawn beam app wasm thread")?;

    thread.join().map_err(|_| Error::InvalidGuestOutput {
        reason: "beam app wasm thread panicked".to_string(),
    })?
}

fn run_guest_command_on_thread(input: CommandRun) -> Result<GuestCommandResult> {
    let bytes = std::fs::read(&input.module_path).context("read beam app wasm module")?;
    app_debug(&format!("wasm module read bytes={}", bytes.len()));
    let module = Module::new(&input.engine, &bytes).context("compile beam app wasm module")?;
    app_debug("wasm module compiled");
    let invocation = GuestInvocation {
        args: input.args,
        host_api_version: HOST_API_VERSION,
        metadata: input.metadata.clone(),
        output_mode: output_mode_label(input.output_mode).to_string(),
    };
    let mut store = Store::new(
        &input.engine,
        HostState::new(
            input.app,
            input.permissions,
            input.metadata,
            input.runtime_handle,
        ),
    );
    store
        .set_fuel(WASM_FUEL)
        .context("set beam app wasm fuel")?;
    store.limiter(|state| &mut state.limits);
    let mut linker = <Linker<HostState>>::new(&input.engine);
    register_host_imports(&mut linker)?;
    let instance = linker
        .instantiate_and_start(&mut store, &module)
        .context("instantiate beam app wasm module")?;
    app_debug("wasm module instantiated");

    let memory = instance
        .get_memory(&store, "memory")
        .ok_or_else(|| Error::MissingWasmExport {
            app: input.manifest_id.clone(),
            export: "memory".to_string(),
        })?;
    let alloc = typed_func::<i32, i32>(&store, &instance, "beam_alloc", &input.manifest_id)?;
    let free = typed_func::<(i32, i32), ()>(&store, &instance, "beam_free", &input.manifest_id)?;
    let main =
        typed_func::<(i32, i32), i64>(&store, &instance, &input.entrypoint, &input.manifest_id)?;
    let input_json = serde_json::to_vec(&invocation).context("serialize beam app invocation")?;
    app_debug(&format!(
        "guest invocation serialized bytes={}",
        input_json.len()
    ));
    let input_ptr = guest_alloc(&mut store, &alloc, input_json.len())?;
    memory
        .write(&mut store, input_ptr, &input_json)
        .context("write beam app invocation")?;
    app_debug("calling guest entrypoint");
    let packed = main
        .call(&mut store, (input_ptr as i32, input_json.len() as i32))
        .context("call beam app command")?;
    app_debug("guest entrypoint returned");
    free.call(&mut store, (input_ptr as i32, input_json.len() as i32))
        .context("free beam app invocation")?;
    read_guest_response(&mut store, &memory, &free, packed)
}

fn read_guest_response(
    store: &mut Store<HostState>,
    memory: &wasmi::Memory,
    free: &wasmi::TypedFunc<(i32, i32), ()>,
    packed: i64,
) -> Result<GuestCommandResult> {
    let (output_ptr, output_len) = unpack_ptr_len(packed)?;
    app_debug(&format!("guest output ptr={output_ptr} bytes={output_len}"));
    if output_len > MAX_GUEST_RESPONSE_BYTES {
        return Err(Error::InvalidGuestOutput {
            reason: format!("guest response too large: {output_len} bytes"),
        });
    }
    let mut output = vec![0_u8; output_len];
    memory
        .read(&*store, output_ptr, &mut output)
        .context("read beam app command output")?;
    free.call(store, (output_ptr as i32, output_len as i32))
        .context("free beam app command output")?;

    let response =
        serde_json::from_slice::<GuestResponse>(&output).context("decode beam app output")?;
    app_debug("guest output decoded");
    match response {
        GuestResponse::ActionPlan { plan } => Ok(GuestCommandResult::ActionPlan(*plan)),
        GuestResponse::Output { value } => {
            let text = value
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("App command completed")
                .to_string();
            Ok(GuestCommandResult::Output(CommandOutput::new(text, value)))
        }
        GuestResponse::Error { message } => Err(Error::GuestCommandFailed { message }),
    }
}

#[derive(Serialize)]
struct GuestInvocation {
    args: Vec<String>,
    host_api_version: u32,
    metadata: HostMetadata,
    output_mode: String,
}

#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
enum GuestResponse {
    ActionPlan { plan: Box<ActionPlan> },
    Output { value: Value },
    Error { message: String },
}

fn output_mode_label(mode: OutputMode) -> &'static str {
    match mode {
        OutputMode::Default => "default",
        OutputMode::Json => "json",
        OutputMode::Yaml => "yaml",
        OutputMode::Markdown => "markdown",
        OutputMode::Compact => "compact",
        OutputMode::Quiet => "quiet",
    }
}
