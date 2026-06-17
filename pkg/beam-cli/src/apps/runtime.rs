// lint-long-file-override allow-max-lines=300
use std::path::Path;

use contextful::ResultContextExt;
use wasmi::{Config, Engine, Linker, Module, Store};

use crate::{
    apps::{
        Error, Result,
        host::HostMetadata,
        model::{AppManifest, InstalledApp},
        store::now,
    },
    runtime::BeamApp,
};

mod command;
mod debug;
mod guest;

pub use command::GuestCommandResult;
use command::{CommandRun, run_guest_command};
use debug::{app_debug, app_debug_enabled};
use guest::{HostState, register_host_imports, typed_func};

const WASM_MAGIC: &[u8; 4] = b"\0asm";
pub(super) const HOST_API_VERSION: u32 = 1;
pub(super) const WASM_FUEL: u64 = 30_000_000;

pub fn validate_wasm_module(app_id: &str, entrypoint: &str, path: &Path) -> Result<()> {
    let bytes = std::fs::read(path).context("read beam app wasm module")?;
    validate_wasm_module_bytes(app_id, entrypoint, &bytes)
}

pub(super) fn validate_wasm_module_bytes(
    app_id: &str,
    entrypoint: &str,
    bytes: &[u8],
) -> Result<()> {
    if bytes.len() < 8 || &bytes[..4] != WASM_MAGIC {
        return Err(Error::InvalidWasmModule {
            app: app_id.to_string(),
        });
    }
    AppRuntime::default().instantiate_for_validation(app_id, entrypoint, bytes)?;

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
        if instance.get_memory(&store, "memory").is_none() {
            return Err(Error::MissingWasmExport {
                app: app_id.to_string(),
                export: "memory".to_string(),
            });
        }
        typed_func::<i32, i32>(&store, &instance, "beam_alloc", app_id)?;
        typed_func::<(i32, i32), ()>(&store, &instance, "beam_free", app_id)?;
        typed_func::<(i32, i32), i64>(&store, &instance, entrypoint, app_id)?;

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
        app_debug(&format!(
            "run command start app={} version={} args={}",
            manifest.id,
            manifest.version,
            args.len()
        ));
        let metadata = self.metadata(app, manifest, installed).await?;
        run_guest_command(CommandRun {
            app: app.clone(),
            args: args.to_vec(),
            engine: self.engine.clone(),
            entrypoint: manifest.wasm.entrypoint.clone(),
            manifest_id: manifest.id.clone(),
            metadata,
            module_path: module_path.to_path_buf(),
            output_mode: app.output_mode,
            permissions: manifest.permissions.clone(),
            runtime_handle: tokio::runtime::Handle::current(),
        })
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
            debug: app_debug_enabled(),
            host_api_version: HOST_API_VERSION,
            manifest_sha256: installed.manifest_sha256.clone(),
            now: now(),
            wallet: format!("{wallet:#x}"),
            wasm_sha256: installed.module_sha256.clone(),
        })
    }
}
