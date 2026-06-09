// lint-long-file-override allow-max-lines=300
use std::path::Path;

use contextful::ResultContextExt;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use wasmi::{Config, Engine, Linker, Module, Store};

use crate::{
    apps::{
        Error, Result,
        host::HostMetadata,
        model::{ActionPlan, AppManifest, InstalledApp},
        store::now,
    },
    output::{CommandOutput, OutputMode},
    runtime::BeamApp,
};

mod guest;

use guest::{HostState, guest_alloc, register_host_imports, typed_func, unpack_ptr_len};

const WASM_MAGIC: &[u8; 4] = b"\0asm";
const HOST_API_VERSION: u32 = 1;
const MAX_GUEST_RESPONSE_BYTES: usize = 2 * 1024 * 1024;
const WASM_FUEL: u64 = 30_000_000;

pub fn validate_wasm_module(app_id: &str, entrypoint: &str, path: &Path) -> Result<()> {
    let bytes = std::fs::read(path).context("read beam app wasm module")?;
    if bytes.len() < 8 || &bytes[..4] != WASM_MAGIC {
        return Err(Error::InvalidWasmModule {
            app: app_id.to_string(),
        });
    }
    AppRuntime::default().instantiate_for_validation(app_id, entrypoint, &bytes)?;

    Ok(())
}

pub struct AppRuntime {
    engine: Engine,
}

impl Default for AppRuntime {
    fn default() -> Self {
        let mut config = Config::default();
        config.consume_fuel(true);
        Self {
            engine: Engine::new(&config),
        }
    }
}

impl AppRuntime {
    fn instantiate_for_validation(
        &self,
        app_id: &str,
        entrypoint: &str,
        bytes: &[u8],
    ) -> Result<()> {
        let module = Module::new(&self.engine, bytes).context("compile beam app wasm module")?;
        let mut store = Store::new(&self.engine, HostState::validation());
        store
            .set_fuel(WASM_FUEL)
            .context("set beam app wasm fuel")?;
        store.limiter(|state| &mut state.limits);
        let mut linker = <Linker<HostState>>::new(&self.engine);
        register_host_imports(&mut linker)?;
        let instance = linker
            .instantiate_and_start(&mut store, &module)
            .context("instantiate beam app wasm module")?;
        if instance.get_func(&store, entrypoint).is_none() {
            return Err(Error::MissingWasmExport {
                app: app_id.to_string(),
                export: entrypoint.to_string(),
            });
        }

        Ok(())
    }

    pub async fn run_command(
        &self,
        app: &BeamApp,
        manifest: &AppManifest,
        installed: &InstalledApp,
        module_path: &Path,
        args: &[String],
    ) -> Result<GuestCommandResult> {
        let bytes = std::fs::read(module_path).context("read beam app wasm module")?;
        let module = Module::new(&self.engine, &bytes).context("compile beam app wasm module")?;
        let metadata = self.metadata(app, manifest, installed).await?;
        let invocation = GuestInvocation {
            args: args.to_vec(),
            host_api_version: HOST_API_VERSION,
            metadata: metadata.clone(),
            output_mode: output_mode_label(app.output_mode).to_string(),
        };
        let mut store = Store::new(
            &self.engine,
            HostState::new(app.clone(), manifest.permissions.clone(), metadata),
        );
        store
            .set_fuel(WASM_FUEL)
            .context("set beam app wasm fuel")?;
        store.limiter(|state| &mut state.limits);
        let mut linker = <Linker<HostState>>::new(&self.engine);
        register_host_imports(&mut linker)?;
        let instance = linker
            .instantiate_and_start(&mut store, &module)
            .context("instantiate beam app wasm module")?;

        let memory =
            instance
                .get_memory(&store, "memory")
                .ok_or_else(|| Error::MissingWasmExport {
                    app: manifest.id.clone(),
                    export: "memory".to_string(),
                })?;
        let alloc = typed_func::<i32, i32>(&store, &instance, "beam_alloc", &manifest.id)?;
        let free = typed_func::<(i32, i32), ()>(&store, &instance, "beam_free", &manifest.id)?;
        let main = typed_func::<(i32, i32), i64>(
            &store,
            &instance,
            &manifest.wasm.entrypoint,
            &manifest.id,
        )?;
        let input = serde_json::to_vec(&invocation).context("serialize beam app invocation")?;
        let input_ptr = guest_alloc(&mut store, &alloc, input.len())?;
        memory
            .write(&mut store, input_ptr, &input)
            .context("write beam app invocation")?;
        let packed = main
            .call(&mut store, (input_ptr as i32, input.len() as i32))
            .context("call beam app command")?;
        free.call(&mut store, (input_ptr as i32, input.len() as i32))
            .context("free beam app invocation")?;
        let (output_ptr, output_len) = unpack_ptr_len(packed)?;
        if output_len > MAX_GUEST_RESPONSE_BYTES {
            return Err(Error::InvalidGuestOutput {
                reason: format!("guest response too large: {output_len} bytes"),
            });
        }
        let mut output = vec![0_u8; output_len];
        memory
            .read(&store, output_ptr, &mut output)
            .context("read beam app command output")?;
        free.call(&mut store, (output_ptr as i32, output_len as i32))
            .context("free beam app command output")?;

        let response =
            serde_json::from_slice::<GuestResponse>(&output).context("decode beam app output")?;
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

    async fn metadata(
        &self,
        app: &BeamApp,
        manifest: &AppManifest,
        installed: &InstalledApp,
    ) -> Result<HostMetadata> {
        let chain = app.active_chain().await.context("resolve beam app chain")?;
        let wallet = app
            .active_address()
            .await
            .context("resolve beam app wallet")?;
        Ok(HostMetadata {
            app_id: manifest.id.clone(),
            app_version: manifest.version.clone(),
            chain: chain.entry.key,
            chain_id: chain.entry.chain_id,
            host_api_version: HOST_API_VERSION,
            manifest_sha256: installed.manifest_sha256.clone(),
            now: now(),
            wallet: format!("{wallet:#x}"),
            wasm_sha256: installed.module_sha256.clone(),
        })
    }
}

#[derive(Debug)]
pub enum GuestCommandResult {
    ActionPlan(ActionPlan),
    Output(CommandOutput),
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
