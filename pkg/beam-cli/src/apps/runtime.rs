use std::path::Path;

use contextful::ResultContextExt;
use wasmi::{Engine, Linker, Module, Store};

use crate::apps::{Error, Result};

const WASM_MAGIC: &[u8; 4] = b"\0asm";

pub fn validate_wasm_module(app_id: &str, entrypoint: &str, path: &Path) -> Result<()> {
    let bytes = std::fs::read(path).context("read beam app wasm module")?;
    if bytes.len() < 8 || &bytes[..4] != WASM_MAGIC {
        return Err(Error::InvalidWasmModule {
            app: app_id.to_string(),
        });
    }
    AppRuntime::default().instantiate(app_id, entrypoint, &bytes)?;

    Ok(())
}

#[derive(Default)]
pub struct AppRuntime {
    engine: Engine,
}

impl AppRuntime {
    fn instantiate(&self, app_id: &str, entrypoint: &str, bytes: &[u8]) -> Result<()> {
        let module = Module::new(&self.engine, bytes).context("compile beam app wasm module")?;
        let mut store = Store::new(&self.engine, HostState);
        let linker = <Linker<HostState>>::new(&self.engine);
        let instance = linker
            .instantiate_and_start(&mut store, &module)
            .context("instantiate beam app wasm module")?;
        if instance.get_func(&store, entrypoint).is_none() {
            return Err(Error::InvalidHostRequest {
                reason: format!("{app_id} wasm missing entrypoint {entrypoint}"),
            });
        }

        Ok(())
    }
}

struct HostState;
