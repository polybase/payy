use serde_json::json;

use crate::{
    cli::util::UtilAction,
    error::Result,
    output::OutputMode,
    runtime::parse_address,
    util::{hash, numbers},
};

use super::render::{print_value, raw_input, structured_input};

pub(super) fn run(output_mode: OutputMode, action: UtilAction) -> Result<()> {
    match action {
        UtilAction::ComputeAddress(args) => {
            let value = hash::compute_address(
                args.address.as_deref(),
                args.nonce.as_deref(),
                args.salt.as_deref(),
                args.init_code.as_deref(),
                args.init_code_hash.as_deref(),
            )?;

            print_value(output_mode, value, json!({ "kind": "compute-address" }))
        }
        UtilAction::Create2(args) => print_value(
            output_mode,
            hash::create2_address(
                args.deployer.as_deref(),
                Some(&args.salt),
                args.init_code.as_deref(),
                args.init_code_hash.as_deref(),
            )?,
            json!({}),
        ),
        UtilAction::HashMessage(args) => print_value(
            output_mode,
            hash::hash_message(&raw_input(args.value, "hash-message")?),
            json!({}),
        ),
        UtilAction::Index(args) => print_value(
            output_mode,
            hash::mapping_index(&args.key_type, &args.key, &args.slot_number)?,
            json!({}),
        ),
        UtilAction::IndexErc7201(args) => print_value(
            output_mode,
            hash::erc7201_index(&structured_input(args.value, "index-erc7201")?),
            json!({}),
        ),
        UtilAction::Keccak(args) => print_value(
            output_mode,
            hash::keccak_hex(&raw_input(args.value, "keccak")?)?,
            json!({}),
        ),
        UtilAction::Namehash(args) => print_value(
            output_mode,
            hash::namehash_hex(&structured_input(args.value, "namehash")?),
            json!({}),
        ),
        UtilAction::Sig(args) => print_value(
            output_mode,
            hash::selector(&structured_input(args.value, "sig")?),
            json!({}),
        ),
        UtilAction::SigEvent(args) => print_value(
            output_mode,
            hash::selector_event(&structured_input(args.value, "sig-event")?),
            json!({}),
        ),
        UtilAction::ToCheckSumAddress(args) => {
            let address = parse_address(&structured_input(args.address, "to-check-sum-address")?)?;
            let chain_id = args
                .chain_id
                .as_deref()
                .map(numbers::parse_u256_value)
                .transpose()?
                .map(|value| value.as_u64());

            print_value(
                output_mode,
                hash::checksum_address(address, chain_id),
                json!({}),
            )
        }
        _ => unreachable!("unexpected util action for hash family"),
    }
}
