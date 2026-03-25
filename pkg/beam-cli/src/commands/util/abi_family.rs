use serde_json::json;

use crate::{cli::util::UtilAction, error::Result, output::OutputMode, util::abi};

use super::render::{
    print_encoded_event, print_named_values, print_value, print_values, structured_input,
};

pub(super) fn run(output_mode: OutputMode, action: UtilAction) -> Result<()> {
    match action {
        UtilAction::AbiEncode(args) => print_value(
            output_mode,
            abi::abi_encode(&args.sig, &args.args)?,
            json!({ "signature": args.sig }),
        ),
        UtilAction::AbiEncodeEvent(args) => {
            print_encoded_event(output_mode, abi::abi_encode_event(&args.sig, &args.args)?)
        }
        UtilAction::Calldata(args) => print_value(
            output_mode,
            abi::calldata(&args.sig, &args.args)?,
            json!({ "signature": args.sig }),
        ),
        UtilAction::DecodeAbi(args) => print_values(
            output_mode,
            abi::decode_abi(
                &args.sig,
                &structured_input(args.calldata, "decode-abi")?,
                args.input,
            )?,
        ),
        UtilAction::DecodeCalldata(args) => print_values(
            output_mode,
            abi::decode_calldata(&args.sig, &structured_input(args.data, "decode-calldata")?)?,
        ),
        UtilAction::DecodeError(args) => print_values(
            output_mode,
            abi::decode_error(&args.sig, &structured_input(args.data, "decode-error")?)?,
        ),
        UtilAction::DecodeEvent(args) => print_named_values(
            output_mode,
            abi::decode_event(
                &args.sig,
                &structured_input(args.data, "decode-event")?,
                &args.topics,
            )?,
        ),
        UtilAction::DecodeString(args) => print_value(
            output_mode,
            abi::decode_string(&structured_input(args.value, "decode-string")?)?,
            json!({}),
        ),
        _ => unreachable!("unexpected util action for abi family"),
    }
}
