use serde_json::json;

use crate::{
    cli::util::UtilAction,
    error::Result,
    output::OutputMode,
    util::{bytes, hash, value_or_stdin_bytes},
};

use super::render::{print_pretty_calldata, print_value, raw_input, structured_input};

pub(super) fn run(output_mode: OutputMode, action: UtilAction) -> Result<()> {
    match action {
        UtilAction::AddressZero => print_value(output_mode, hash::address_zero(), json!({})),
        UtilAction::ConcatHex(args) => print_value(
            output_mode,
            bytes::concat_hex(&args.values)?,
            json!({ "inputs": args.values }),
        ),
        UtilAction::FormatBytes32String(args) => print_value(
            output_mode,
            bytes::format_bytes32_string(&raw_input(args.value, "format-bytes32-string")?)?,
            json!({}),
        ),
        UtilAction::FromBin(args) => print_value(
            output_mode,
            bytes::hex_encode(&value_or_stdin_bytes(args.value, "from-bin")?),
            json!({}),
        ),
        UtilAction::FromUtf8(args) => print_value(
            output_mode,
            bytes::utf8_to_hex(&raw_input(args.value, "from-utf8")?),
            json!({}),
        ),
        UtilAction::HashZero => print_value(output_mode, hash::hash_zero(), json!({})),
        UtilAction::Pad(args) => print_value(
            output_mode,
            bytes::pad_hex(&structured_input(args.data, "pad")?, args.len, args.right)?,
            json!({}),
        ),
        UtilAction::ParseBytes32Address(args) => print_value(
            output_mode,
            bytes::parse_bytes32_address(&structured_input(args.value, "parse-bytes32-address")?)?,
            json!({}),
        ),
        UtilAction::ParseBytes32String(args) => print_value(
            output_mode,
            bytes::parse_bytes32_string(&structured_input(args.value, "parse-bytes32-string")?)?,
            json!({}),
        ),
        UtilAction::PrettyCalldata(args) => print_pretty_calldata(
            output_mode,
            bytes::pretty_calldata(&structured_input(args.value, "pretty-calldata")?)?,
        ),
        UtilAction::ToAscii(args) => print_value(
            output_mode,
            bytes::hex_to_ascii(&structured_input(args.value, "to-ascii")?)?,
            json!({}),
        ),
        UtilAction::ToBytes32(args) => print_value(
            output_mode,
            bytes::to_bytes32(&structured_input(args.value, "to-bytes32")?)?,
            json!({}),
        ),
        UtilAction::ToHexdata(args) => print_value(
            output_mode,
            bytes::normalize_hexdata(&structured_input(args.value, "to-hexdata")?)?,
            json!({}),
        ),
        UtilAction::ToUtf8(args) => print_value(
            output_mode,
            bytes::hex_to_utf8(&structured_input(args.value, "to-utf8")?)?,
            json!({}),
        ),
        _ => unreachable!("unexpected util action for bytes family"),
    }
}
