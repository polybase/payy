// lint-long-file-override allow-max-lines=300
use std::io::IsTerminal;

use serde_json::{Value, json};

use crate::{
    human_output::sanitize_control_chars,
    output::{CommandOutput, OutputMode},
};

use super::{
    error::Error,
    proxy::{ProxyInfo, ProxyStatus, proxy_value},
    render::common_target_value,
    sourcify::{
        compilation_lines, match_summary, sourcify_not_checked_value, sourcify_record_value,
        sourcify_status_value,
    },
    target::{BytecodeInfo, InspectionTarget},
};

pub(super) fn not_runtime_code_output(
    target: &InspectionTarget,
    bytecode: &BytecodeInfo,
) -> CommandOutput {
    let mut value = common_target_value(target);
    value.insert("kind".to_owned(), json!("no_runtime_code"));
    value.insert(
        "bytecode".to_owned(),
        json!({
            "byte_len": bytecode.byte_len,
            "code_hash": bytecode.code_hash,
        }),
    );
    value.insert("sourcify".to_owned(), sourcify_not_checked_value());
    value.insert(
        "proxy".to_owned(),
        proxy_value(&ProxyInfo {
            implementations: Vec::new(),
            proxy_type: None,
            status: ProxyStatus::NotPerformed,
        }),
    );

    CommandOutput::new(
        format!(
            "Chain: {} ({})\nAddress: {}\nKind: no_runtime_code\nRuntime bytecode: 0 bytes\nCode hash: {}\n\nSourcify: not checked",
            target.chain, target.chain_id, target.checksum_address, bytecode.code_hash
        ),
        Value::Object(value),
    )
    .compact(format!(
        "no_runtime_code {} sourcify=not_checked",
        target.checksum_address
    ))
    .markdown(format!(
        "Chain: {} ({})\n\nAddress: `{}`\n\nKind: `no_runtime_code`\n\nRuntime bytecode: 0 bytes\n\nCode hash: `{}`\n\nSourcify: not checked",
        target.chain, target.chain_id, target.checksum_address, bytecode.code_hash
    ))
}

pub(super) fn verified_output(
    target: &InspectionTarget,
    bytecode: &BytecodeInfo,
    record: &sourcify_interface::ContractRecord,
    proxy: &ProxyInfo,
) -> CommandOutput {
    let mut lines = info_base_lines(target, bytecode);
    lines.push(String::new());
    lines.push("Sourcify: runtime verified".to_owned());
    lines.push(format!("Match: {}", match_summary(record)));
    lines.extend(compilation_lines(record));
    lines.extend(proxy_lines(proxy));

    let mut value = common_target_value(target);
    value.insert("kind".to_owned(), json!("contract"));
    value.insert(
        "bytecode".to_owned(),
        json!({
            "byte_len": bytecode.byte_len,
            "code_hash": bytecode.code_hash,
        }),
    );
    value.insert("sourcify".to_owned(), sourcify_record_value(record));
    value.insert("proxy".to_owned(), proxy_value(proxy));

    CommandOutput::new(lines.join("\n"), Value::Object(value))
        .compact(format!(
            "contract {} sourcify=runtime_verified",
            target.checksum_address
        ))
        .markdown(lines.join("\n\n"))
}

pub(super) fn sourcify_status_output(
    target: &InspectionTarget,
    bytecode: &BytecodeInfo,
    status: &str,
    error: Option<&str>,
    proxy: &ProxyInfo,
) -> CommandOutput {
    let mut lines = info_base_lines(target, bytecode);
    lines.push(String::new());
    lines.push(format!("Sourcify: {}", human_sourcify_status(status)));

    let mut value = common_target_value(target);
    value.insert("kind".to_owned(), json!("contract"));
    value.insert(
        "bytecode".to_owned(),
        json!({
            "byte_len": bytecode.byte_len,
            "code_hash": bytecode.code_hash,
        }),
    );
    value.insert("sourcify".to_owned(), sourcify_status_value(status, error));
    value.insert("proxy".to_owned(), proxy_value(proxy));

    CommandOutput::new(lines.join("\n"), Value::Object(value))
        .compact(format!(
            "contract {} sourcify={status}",
            target.checksum_address
        ))
        .markdown(lines.join("\n\n"))
}

pub(super) fn failed_proxy() -> ProxyInfo {
    ProxyInfo {
        implementations: Vec::new(),
        proxy_type: None,
        status: ProxyStatus::Failed,
    }
}

pub(super) fn source_summary_output(
    target: &InspectionTarget,
    record: &sourcify_interface::ContractRecord,
    files: Vec<String>,
    proxy: &ProxyInfo,
) -> CommandOutput {
    let mut lines = vec!["Sourcify: runtime verified".to_owned()];
    lines.extend(compilation_lines(record));
    lines.push("Files:".to_owned());
    lines.extend(
        files
            .iter()
            .map(|file| format!("  {}", sanitize_control_chars(file))),
    );
    let compact = files
        .iter()
        .map(|file| sanitize_control_chars(file))
        .collect::<Vec<_>>()
        .join("\n");

    let mut value = common_target_value(target);
    value.insert("sourcify".to_owned(), sourcify_record_value(record));
    value.insert("files".to_owned(), json!(files));
    value.insert("proxy".to_owned(), proxy_value(proxy));

    CommandOutput::new(lines.join("\n"), Value::Object(value))
        .compact(compact)
        .markdown(lines.join("\n\n"))
}

pub(super) fn error_code(err: &Error) -> &'static str {
    match err {
        Error::SourcifyLookupFailed { .. } => "sourcify_lookup_failed",
        Error::SourcifyResponseTooLarge { .. } => "sourcify_response_too_large",
        Error::SourcifyMalformedResponse { .. } => "sourcify_malformed_response",
        _ => "sourcify_lookup_failed",
    }
}

pub(super) fn print_proxy_tip(
    mode: OutputMode,
    artifact: &str,
    proxy: &ProxyInfo,
    destination: Option<&str>,
) {
    if mode != OutputMode::Default || !std::io::stderr().is_terminal() {
        return;
    }
    if proxy.status != ProxyStatus::Resolved {
        return;
    }
    for implementation in &proxy.implementations {
        match destination {
            Some(destination) => eprintln!(
                "Tip: this address appears to be a proxy. Implementation: {}\n     To export the implementation: beam contract export {} {destination}",
                implementation.address, implementation.address
            ),
            None => eprintln!(
                "Tip: this address appears to be a proxy. Implementation: {}\n     To fetch the implementation {artifact}: beam contract {} {}",
                implementation.address,
                artifact.to_ascii_lowercase(),
                implementation.address
            ),
        }
    }
}

pub(super) fn print_sourcify_miss_tip(
    mode: OutputMode,
    target: &InspectionTarget,
    err: &Error,
    runtime_checked: bool,
    rpc_failure: Option<&str>,
) {
    if mode != OutputMode::Default || !std::io::stderr().is_terminal() {
        return;
    }

    match err {
        Error::SourcifyNotVerified { artifact, .. } if artifact == "ABI" => {
            eprintln!("ABI not found on Sourcify for {}", target.checksum_address);
        }
        Error::SourcifyNotVerified { .. } => {
            eprintln!(
                "Source not found on Sourcify for {}",
                target.checksum_address
            );
        }
        Error::SourcifyRuntimeNotVerified { artifact, .. } if artifact == "ABI" => {
            eprintln!("ABI not found on Sourcify for {}", target.checksum_address);
        }
        Error::SourcifyRuntimeNotVerified { artifact, .. } => {
            eprintln!(
                "{artifact} not runtime-verified on Sourcify for {}",
                target.checksum_address
            );
        }
        _ => return,
    }

    if runtime_checked {
        eprintln!(
            "Tip: deployed bytecode may still be available with:\n     beam contract bytecode {}",
            target.checksum_address
        );
    } else {
        eprintln!("Runtime code was not checked.");
        if let Some(reason) = rpc_failure {
            eprintln!("RPC check failed: {reason}");
        }
    }
}

fn info_base_lines(target: &InspectionTarget, bytecode: &BytecodeInfo) -> Vec<String> {
    vec![
        format!("Chain: {} ({})", target.chain, target.chain_id),
        format!("Address: {}", target.checksum_address),
        "Kind: contract".to_owned(),
        format!("Runtime bytecode: {} bytes", bytecode.byte_len),
        format!("Code hash: {}", bytecode.code_hash),
    ]
}

fn proxy_lines(proxy: &ProxyInfo) -> Vec<String> {
    let mut lines = vec![String::new()];
    match proxy.status {
        ProxyStatus::Failed => return vec!["Proxy: lookup failed".to_owned()],
        ProxyStatus::NotPerformed => return vec!["Proxy: not checked".to_owned()],
        ProxyStatus::NotProxy => return vec!["Proxy: no".to_owned()],
        ProxyStatus::Resolved => lines.push("Proxy: yes".to_owned()),
    }
    if let Some(proxy_type) = proxy.proxy_type.as_ref() {
        lines.push(format!(
            "Proxy type: {}",
            sanitize_control_chars(proxy_type)
        ));
    }
    for implementation in &proxy.implementations {
        lines.push(format!("Implementation: {}", implementation.address));
        if let Some(name) = implementation.name.as_ref() {
            lines.push(format!(
                "Implementation name: {}",
                sanitize_control_chars(name)
            ));
        }
    }
    lines
}

fn human_sourcify_status(status: &str) -> &'static str {
    match status {
        "not_verified" | "runtime_not_verified" => "not runtime verified",
        "unsupported_chain" => "unsupported chain",
        "lookup_failed" => "lookup failed",
        _ => "not checked",
    }
}
