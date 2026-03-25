use serde_json::{Value, json};

use crate::{
    error::{Error, Result},
    output::{CommandOutput, OutputMode},
    util::{self, abi::EncodedEvent, bytes::PrettyCalldata},
};

pub(crate) fn print_encoded_event(output_mode: OutputMode, encoded: EncodedEvent) -> Result<()> {
    let mut lines = encoded
        .topics
        .iter()
        .enumerate()
        .map(|(index, topic)| format!("[topic{index}]: {topic}"))
        .collect::<Vec<_>>();
    lines.push(format!("[data]: {}", encoded.data));

    CommandOutput::new(
        lines.join("\n"),
        json!({
            "data": encoded.data,
            "topics": encoded.topics,
        }),
    )
    .print(output_mode)
}

pub(crate) fn print_json(output_mode: OutputMode, value: Value) -> Result<()> {
    CommandOutput::new(render_value(&value), value).print(output_mode)
}

pub(crate) fn print_named_values(
    output_mode: OutputMode,
    values: Vec<(String, Value)>,
) -> Result<()> {
    let default = values
        .iter()
        .map(|(name, value)| format!("{name}: {}", render_value(value)))
        .collect::<Vec<_>>()
        .join("\n");
    let json_values = values
        .into_iter()
        .collect::<serde_json::Map<String, Value>>();

    CommandOutput::new(default, Value::Object(json_values)).print(output_mode)
}

pub(crate) fn print_pretty_calldata(
    output_mode: OutputMode,
    calldata: PrettyCalldata,
) -> Result<()> {
    let mut lines = Vec::new();
    if let Some(selector) = calldata.selector.as_ref() {
        lines.push(format!("Selector: {selector}"));
    }
    lines.extend(
        calldata
            .words
            .iter()
            .enumerate()
            .map(|(index, word)| format!("Word {index}: {word}")),
    );
    if let Some(remainder) = calldata.remainder.as_ref() {
        lines.push(format!("Remainder: {remainder}"));
    }

    CommandOutput::new(
        lines.join("\n"),
        json!({
            "remainder": calldata.remainder,
            "selector": calldata.selector,
            "words": calldata.words,
        }),
    )
    .print(output_mode)
}

pub(crate) fn print_value(output_mode: OutputMode, value: String, extra: Value) -> Result<()> {
    let mut object = serde_json::Map::new();
    object.insert("value".to_string(), Value::String(value.clone()));
    if let Value::Object(extra) = extra {
        object.extend(extra);
    }

    CommandOutput::new(value.clone(), Value::Object(object))
        .compact(value)
        .print(output_mode)
}

pub(crate) fn print_values(output_mode: OutputMode, values: Vec<Value>) -> Result<()> {
    CommandOutput::new(
        values
            .iter()
            .map(render_value)
            .collect::<Vec<_>>()
            .join("\n"),
        json!({ "values": values }),
    )
    .print(output_mode)
}

pub(crate) fn raw_input(value: Option<String>, command: &str) -> Result<String> {
    util::value_or_stdin_text(value, command)
}

pub(crate) fn required_field(value: Option<String>, command: &str) -> Result<String> {
    value.ok_or_else(|| Error::MissingUtilInput {
        command: command.to_string(),
    })
}

pub(crate) fn structured_input(value: Option<String>, command: &str) -> Result<String> {
    Ok(util::value_or_stdin_text(value, command)?
        .trim()
        .to_string())
}

fn render_value(value: &Value) -> String {
    match value {
        Value::String(value) => value.clone(),
        Value::Array(_) | Value::Object(_) => {
            serde_json::to_string(value).unwrap_or_else(|_| value.to_string())
        }
        _ => value.to_string(),
    }
}
