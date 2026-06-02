use std::io::Write;

use contextful::ResultContextExt;
use serde_json::{Map, Value, json};

use crate::{
    error::Result,
    output::{CommandOutput, OutputMode},
};

use super::{
    proxy::{ProxyInfo, proxy_value},
    source::SourceMatchKind,
    sourcify::sourcify_record_value,
    target::{BytecodeInfo, InspectionTarget},
};

pub(super) fn common_target_value(target: &InspectionTarget) -> Map<String, Value> {
    let mut object = Map::new();
    object.insert("chain".to_owned(), json!(target.chain));
    object.insert("chain_id".to_owned(), json!(target.chain_id));
    object.insert("address".to_owned(), json!(target.checksum_address));
    object.insert("input_address".to_owned(), json!(target.input_address));
    object
}

pub(super) fn print_raw_text(mode: OutputMode, text: &str) -> Result<()> {
    match mode {
        OutputMode::Default | OutputMode::Compact => {
            let mut stdout = std::io::stdout().lock();
            stdout
                .write_all(text.as_bytes())
                .context("write beam contract raw output")?;
            if !text.ends_with('\n') {
                stdout
                    .write_all(b"\n")
                    .context("write beam contract raw output newline")?;
            }
            stdout.flush().context("flush beam contract raw output")?;
        }
        OutputMode::Quiet => {}
        OutputMode::Json | OutputMode::Yaml | OutputMode::Markdown => unreachable!(),
    }

    Ok(())
}

pub(super) fn bytecode_output(
    target: &InspectionTarget,
    block: &str,
    bytecode: &BytecodeInfo,
) -> CommandOutput {
    let mut value = common_target_value(target);
    value.insert("block".to_owned(), json!(block));
    value.insert("bytecode".to_owned(), json!(bytecode.hex));
    value.insert("byte_len".to_owned(), json!(bytecode.byte_len));
    value.insert("code_hash".to_owned(), json!(bytecode.code_hash));
    value.insert(
        "proxy".to_owned(),
        json!({
            "status": "not_performed",
            "implementations": [],
        }),
    );

    CommandOutput::new(bytecode.hex.clone(), Value::Object(value))
        .compact(bytecode.hex.clone())
        .markdown(format!("```text\n{}\n```", bytecode.hex))
}

pub(super) fn abi_output(
    target: &InspectionTarget,
    abi_text: &str,
    abi: Vec<Value>,
    record: &sourcify_interface::ContractRecord,
    proxy: &ProxyInfo,
) -> CommandOutput {
    let mut value = common_target_value(target);
    value.insert("abi".to_owned(), Value::Array(abi));
    value.insert("sourcify".to_owned(), sourcify_record_value(record));
    value.insert("proxy".to_owned(), proxy_value(proxy));

    CommandOutput::new(abi_text.to_owned(), Value::Object(value))
        .compact(abi_text.to_owned())
        .markdown(format!("```json\n{abi_text}\n```"))
}

pub(super) fn source_file_output(
    target: &InspectionTarget,
    path: &str,
    kind: &SourceMatchKind,
    content: &str,
    record: &sourcify_interface::ContractRecord,
    proxy: &ProxyInfo,
) -> CommandOutput {
    let mut value = common_target_value(target);
    value.insert("path".to_owned(), json!(path));
    value.insert("matched_by".to_owned(), json!(kind.as_str()));
    value.insert("content".to_owned(), json!(content));
    value.insert("sourcify".to_owned(), sourcify_record_value(record));
    value.insert("proxy".to_owned(), proxy_value(proxy));

    CommandOutput::new(content.to_owned(), Value::Object(value))
        .compact(content.to_owned())
        .markdown(format!("```text\n{content}\n```"))
}

pub(super) fn export_output(
    target: &InspectionTarget,
    destination: &str,
    written_files: &[String],
    record: &sourcify_interface::ContractRecord,
    proxy: &ProxyInfo,
) -> CommandOutput {
    let mut value = common_target_value(target);
    value.insert("destination".to_owned(), json!(destination));
    value.insert("sourcify".to_owned(), sourcify_record_value(record));
    value.insert("written_files".to_owned(), json!(written_files));
    value.insert("proxy".to_owned(), proxy_value(proxy));

    let default = format!(
        "Exported verified contract bundle to {destination}\nFiles: {}",
        written_files.len()
    );

    CommandOutput::new(default.clone(), Value::Object(value))
        .compact(destination.to_owned())
        .markdown(default)
}
