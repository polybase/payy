// lint-long-file-override allow-max-lines=300
use contextful::ResultContextExt;
use serde_json::Value;
use tokio::runtime::Handle;
use wasmi::{
    Caller, Extern, Instance, Linker, Memory, Store, StoreLimits, StoreLimitsBuilder, TypedFunc,
    WasmParams, WasmResults,
};

use crate::{
    apps::{
        Error, Result, format_error_chain,
        host::{HostCallResponse, HostMetadata, HostRequest, handle_host_request},
        model::AppPermissions,
    },
    runtime::BeamApp,
};

use super::debug::{app_debug, app_debug_enabled, host_request_summary, host_value_summary};

const MAX_WASM_MEMORY_BYTES: usize = 64 * 1024 * 1024;

pub(super) struct HostState {
    app: Option<BeamApp>,
    diagnostics: Vec<Value>,
    pub(super) limits: StoreLimits,
    metadata: Option<HostMetadata>,
    permissions: Option<AppPermissions>,
    runtime_handle: Option<Handle>,
    structured_output: Option<Value>,
}

impl HostState {
    pub(super) fn new(
        app: BeamApp,
        permissions: AppPermissions,
        metadata: HostMetadata,
        runtime_handle: Handle,
    ) -> Self {
        Self {
            app: Some(app),
            diagnostics: Vec::new(),
            limits: StoreLimitsBuilder::new()
                .memory_size(MAX_WASM_MEMORY_BYTES)
                .build(),
            metadata: Some(metadata),
            permissions: Some(permissions),
            runtime_handle: Some(runtime_handle),
            structured_output: None,
        }
    }

    pub(super) fn validation() -> Self {
        Self {
            app: None,
            diagnostics: Vec::new(),
            limits: StoreLimitsBuilder::new()
                .memory_size(MAX_WASM_MEMORY_BYTES)
                .build(),
            metadata: None,
            permissions: None,
            runtime_handle: None,
            structured_output: None,
        }
    }
}

pub(super) fn register_host_imports(linker: &mut Linker<HostState>) -> Result<()> {
    linker
        .func_wrap(
            "env",
            "beam_host_call",
            |caller: Caller<HostState>,
             request_ptr: i32,
             request_len: i32,
             response_ptr: i32,
             response_capacity: i32|
             -> i32 {
                host_call(
                    caller,
                    request_ptr,
                    request_len,
                    response_ptr,
                    response_capacity,
                )
                .unwrap_or_else(|error| {
                    serde_json::to_vec(&HostCallResponse::error(error.to_string()))
                        .ok()
                        .and_then(|bytes| i32::try_from(bytes.len()).ok())
                        .map(|len| -len)
                        .unwrap_or(i32::MIN)
                })
            },
        )
        .context("register beam app host call")?;
    Ok(())
}

pub(super) fn typed_func<Params, Results>(
    store: &Store<HostState>,
    instance: &Instance,
    name: &str,
    app_id: &str,
) -> Result<TypedFunc<Params, Results>>
where
    Params: WasmParams,
    Results: WasmResults,
{
    let func = instance
        .get_func(store, name)
        .ok_or_else(|| Error::MissingWasmExport {
            app: app_id.to_string(),
            export: name.to_string(),
        })?;
    Ok(func.typed(store).context("type beam app wasm export")?)
}

pub(super) fn guest_alloc(
    store: &mut Store<HostState>,
    alloc: &TypedFunc<i32, i32>,
    len: usize,
) -> Result<usize> {
    let len = i32::try_from(len).map_err(|_| Error::InvalidHostRequest {
        reason: format!("guest allocation too large: {len} bytes"),
    })?;
    let ptr = alloc
        .call(store, len)
        .context("allocate beam app guest memory")?;
    checked_ptr(ptr, "guest allocation pointer")
}

pub(super) fn unpack_ptr_len(value: i64) -> Result<(usize, usize)> {
    let value = u64::try_from(value).map_err(|_| Error::InvalidGuestOutput {
        reason: format!("negative guest pointer/length result: {value}"),
    })?;
    let ptr = (value >> 32) as u32;
    let len = (value & 0xffff_ffff) as u32;
    Ok((ptr as usize, len as usize))
}

fn host_call(
    mut caller: Caller<HostState>,
    request_ptr: i32,
    request_len: i32,
    response_ptr: i32,
    response_capacity: i32,
) -> Result<i32> {
    let request_len = checked_len(request_len, "request length")?;
    let response_capacity = checked_len(response_capacity, "response capacity")?;
    let memory = caller_memory(&caller)?;
    let mut request_bytes = vec![0_u8; request_len];
    memory
        .read(
            &caller,
            checked_ptr(request_ptr, "request pointer")?,
            &mut request_bytes,
        )
        .context("read beam app host request")?;
    let request = serde_json::from_slice::<HostRequest>(&request_bytes)
        .context("decode beam app host request")?;
    let request_summary = app_debug_enabled().then(|| host_request_summary(&request));
    if let Some(summary) = &request_summary {
        app_debug(&format!("host call start {summary}"));
    }
    let app = caller
        .data()
        .app
        .clone()
        .ok_or_else(|| Error::InvalidHostRequest {
            reason: "host call is unavailable during validation".to_string(),
        })?;
    let permissions =
        caller
            .data()
            .permissions
            .clone()
            .ok_or_else(|| Error::InvalidHostRequest {
                reason: "host permissions missing".to_string(),
            })?;
    let metadata = caller
        .data()
        .metadata
        .clone()
        .ok_or_else(|| Error::InvalidHostRequest {
            reason: "host metadata missing".to_string(),
        })?;
    let mut structured_output = caller.data().structured_output.clone();
    let mut diagnostics = caller.data().diagnostics.clone();
    let runtime_handle =
        caller
            .data()
            .runtime_handle
            .clone()
            .ok_or_else(|| Error::InvalidHostRequest {
                reason: "host runtime handle missing".to_string(),
            })?;
    let result = runtime_handle.block_on(handle_host_request(
        &app,
        &permissions,
        &metadata,
        request,
        &mut structured_output,
        &mut diagnostics,
    ));
    caller.data_mut().structured_output = structured_output;
    caller.data_mut().diagnostics = diagnostics;
    if let Some(summary) = &request_summary {
        match &result {
            Ok(value) => app_debug(&format!(
                "host call ok {summary}; {}",
                host_value_summary(value)
            )),
            Err(error) => app_debug(&format!("host call error {summary}; {error}")),
        }
    }
    let response = match result {
        Ok(value) => HostCallResponse::ok(value),
        Err(error) => HostCallResponse::error(format_error_chain(&error)),
    };
    let response = serde_json::to_vec(&response).context("serialize beam app host response")?;
    if response.len() > response_capacity {
        return Ok(-i32::try_from(response.len()).unwrap_or(i32::MAX));
    }
    memory
        .write(
            &mut caller,
            checked_ptr(response_ptr, "response pointer")?,
            &response,
        )
        .context("write beam app host response")?;
    Ok(i32::try_from(response.len()).unwrap_or(i32::MAX))
}

fn caller_memory(caller: &Caller<HostState>) -> Result<Memory> {
    match caller.get_export("memory") {
        Some(Extern::Memory(memory)) => Ok(memory),
        _ => Err(Error::MissingWasmExport {
            app: "guest".to_string(),
            export: "memory".to_string(),
        }),
    }
}

fn checked_ptr(value: i32, field: &str) -> Result<usize> {
    usize::try_from(value).map_err(|_| Error::InvalidHostRequest {
        reason: format!("invalid {field}: {value}"),
    })
}

fn checked_len(value: i32, field: &str) -> Result<usize> {
    usize::try_from(value).map_err(|_| Error::InvalidHostRequest {
        reason: format!("invalid {field}: {value}"),
    })
}
