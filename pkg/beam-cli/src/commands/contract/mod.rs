// lint-long-file-override allow-max-lines=300
mod artifact;
mod error;
mod export;
mod info;
mod proxy;
mod render;
mod source;
mod sourcify;
mod target;

#[cfg(test)]
mod tests;

use serde_json::Value;
use sourcify_client_reqwest::{SourcifyReqwestClient, SourcifyReqwestClientOptions};
use sourcify_interface::SourcifyClient;
use std::io::IsTerminal;

use crate::{
    cli::{
        ContractAction, ContractAddressArgs, ContractBytecodeArgs, ContractExportArgs,
        ContractSourceArgs,
    },
    error::{Error as BeamError, Result},
    human_output::sanitize_control_chars,
    output::{OutputMode, with_loading},
    runtime::BeamApp,
};

use self::{
    artifact::runtime_verified_artifact,
    error::Error,
    export::export_bundle,
    info::{
        error_code, failed_proxy, not_runtime_code_output, print_proxy_tip, source_summary_output,
        sourcify_status_output, verified_output,
    },
    proxy::parse_proxy,
    render::{abi_output, bytecode_output, export_output, print_raw_text, source_file_output},
    source::{match_source_path, require_sources, source_paths},
    sourcify::{
        ABI_CAP_BYTES, Artifact, EXPORT_CAP_BYTES, INFO_CAP_BYTES, SOURCE_CAP_BYTES, abi_fields,
        export_fields, info_fields, lookup_contract, source_fields,
    },
    target::{build_target, fetch_bytecode, parse_block, required_rpc_client},
};

pub async fn run(app: &BeamApp, action: ContractAction) -> Result<()> {
    let client =
        SourcifyReqwestClient::new(SourcifyReqwestClientOptions::Public).map_err(|err| {
            BeamError::ContractSourcifyLookupFailed {
                address: "unknown".to_owned(),
                reason: err.to_string(),
            }
        })?;
    run_with_client(app, action, &client).await
}

async fn run_with_client(
    app: &BeamApp,
    action: ContractAction,
    sourcify_client: &dyn SourcifyClient,
) -> Result<()> {
    match action {
        ContractAction::Info(args) => run_info(app, args, sourcify_client).await,
        ContractAction::Bytecode(args) => run_bytecode(app, args).await,
        ContractAction::Abi(args) => run_abi(app, args, sourcify_client).await,
        ContractAction::Source(args) => run_source(app, args, sourcify_client).await,
        ContractAction::Export(args) => run_export(app, args, sourcify_client).await,
    }
}

async fn run_info(
    app: &BeamApp,
    args: ContractAddressArgs,
    sourcify_client: &dyn SourcifyClient,
) -> Result<()> {
    let target = build_target(app, &args.address).await?;
    let client = required_rpc_client(app, &target).await?;
    let bytecode = with_loading(app.output_mode, "Fetching contract bytecode...", async {
        fetch_bytecode(&client, target.address, web3::types::BlockNumber::Latest)
            .await
            .map_err(BeamError::from)
    })
    .await?;

    if bytecode.byte_len == 0 {
        return not_runtime_code_output(&target, &bytecode).print(app.output_mode);
    }

    let response = lookup_contract(sourcify_client, &target, info_fields(), INFO_CAP_BYTES).await;
    let output = match response {
        Ok(response) if response.contract.runtime_match.is_some() => {
            let proxy = parse_proxy(&response.contract, true);
            verified_output(&target, &bytecode, &response.contract, &proxy)
        }
        Ok(response) => {
            let proxy = parse_proxy(&response.contract, true);
            sourcify_status_output(&target, &bytecode, "runtime_not_verified", None, &proxy)
        }
        Err(Error::SourcifyNotVerified { .. }) => {
            sourcify_status_output(&target, &bytecode, "not_verified", None, &failed_proxy())
        }
        Err(Error::SourcifyRuntimeNotVerified { .. }) => sourcify_status_output(
            &target,
            &bytecode,
            "runtime_not_verified",
            None,
            &failed_proxy(),
        ),
        Err(Error::SourcifyChainUnsupported { .. }) => sourcify_status_output(
            &target,
            &bytecode,
            "unsupported_chain",
            None,
            &failed_proxy(),
        ),
        Err(err @ Error::SourcifyLookupFailed { .. })
        | Err(err @ Error::SourcifyResponseTooLarge { .. })
        | Err(err @ Error::SourcifyMalformedResponse { .. }) => {
            let proxy = failed_proxy();
            sourcify_status_output(
                &target,
                &bytecode,
                "lookup_failed",
                Some(error_code(&err)),
                &proxy,
            )
        }
        Err(err) => return Err(err.into()),
    };

    output.print(app.output_mode)
}

async fn run_bytecode(app: &BeamApp, args: ContractBytecodeArgs) -> Result<()> {
    let target = build_target(app, &args.address).await?;
    let block = parse_block(args.block.as_deref())?;
    let client = required_rpc_client(app, &target).await?;
    let bytecode = with_loading(app.output_mode, "Fetching contract bytecode...", async {
        fetch_bytecode(&client, target.address, block.number)
            .await
            .map_err(BeamError::from)
    })
    .await?;

    bytecode_output(&target, &block.selector, &bytecode).print(app.output_mode)
}

async fn run_abi(
    app: &BeamApp,
    args: ContractAddressArgs,
    sourcify_client: &dyn SourcifyClient,
) -> Result<()> {
    let target = build_target(app, &args.address).await?;
    let response = runtime_verified_artifact(
        app,
        sourcify_client,
        &target,
        abi_fields(),
        ABI_CAP_BYTES,
        Artifact::Abi,
    )
    .await?;
    let abi = response
        .contract
        .abi
        .clone()
        .ok_or_else(|| Error::SourcifyMalformedResponse {
            reason: "abi field is missing".to_owned(),
        })?;
    let abi_text = serde_json::to_string_pretty(&Value::Array(abi.clone())).map_err(|err| {
        Error::SourcifyMalformedResponse {
            reason: err.to_string(),
        }
    })?;
    let proxy = parse_proxy(&response.contract, true);
    print_proxy_tip(app.output_mode, "abi", &proxy, None);

    abi_output(&target, &abi_text, abi, &response.contract, &proxy).print(app.output_mode)
}

async fn run_source(
    app: &BeamApp,
    args: ContractSourceArgs,
    sourcify_client: &dyn SourcifyClient,
) -> Result<()> {
    let target = build_target(app, &args.address).await?;
    let response = runtime_verified_artifact(
        app,
        sourcify_client,
        &target,
        source_fields(),
        SOURCE_CAP_BYTES,
        Artifact::Source,
    )
    .await?;
    let sources = require_sources(response.contract.sources.as_ref())?;
    let proxy = parse_proxy(&response.contract, true);

    let Some(source_path) = args.source_path.as_ref() else {
        print_proxy_tip(app.output_mode, "source", &proxy, None);
        return source_summary_output(&target, &response.contract, source_paths(sources), &proxy)
            .print(app.output_mode);
    };

    let matched = match match_source_path(sources, source_path) {
        Ok(matched) => matched,
        Err(err @ Error::SourcePathAmbiguous { .. }) => {
            print_source_path_ambiguity(app.output_mode, &err);
            return Err(err.into());
        }
        Err(err) => return Err(err.into()),
    };
    print_proxy_tip(app.output_mode, "source", &proxy, None);
    match app.output_mode {
        OutputMode::Default | OutputMode::Compact => {
            print_raw_text(app.output_mode, matched.content)
        }
        OutputMode::Quiet => Ok(()),
        OutputMode::Json | OutputMode::Yaml | OutputMode::Markdown => source_file_output(
            &target,
            &matched.path,
            &matched.kind,
            matched.content,
            &response.contract,
            &proxy,
        )
        .print(app.output_mode),
    }
}

async fn run_export(
    app: &BeamApp,
    args: ContractExportArgs,
    sourcify_client: &dyn SourcifyClient,
) -> Result<()> {
    let target = build_target(app, &args.address).await?;
    let response = runtime_verified_artifact(
        app,
        sourcify_client,
        &target,
        export_fields(),
        EXPORT_CAP_BYTES,
        Artifact::Export,
    )
    .await?;
    let proxy = parse_proxy(&response.contract, true);
    let exported = export_bundle(&target, &response, &args.destination)?;
    print_proxy_tip(app.output_mode, "export", &proxy, Some(&args.destination));

    export_output(
        &target,
        &exported.destination,
        &exported.written_files,
        &response.contract,
        &proxy,
    )
    .print(app.output_mode)
}

fn print_source_path_ambiguity(mode: OutputMode, err: &Error) {
    if mode != OutputMode::Default || !std::io::stderr().is_terminal() {
        return;
    }
    let Error::SourcePathAmbiguous { matches, .. } = err else {
        return;
    };
    eprintln!("Matching source paths:");
    for path in matches {
        eprintln!("  {}", sanitize_control_chars(path));
    }
}
