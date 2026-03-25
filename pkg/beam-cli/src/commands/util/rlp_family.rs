use crate::{cli::util::UtilAction, error::Result, output::OutputMode, util::rlp};

use super::render::{print_json, print_value, structured_input};

pub(super) fn run(output_mode: OutputMode, action: UtilAction) -> Result<()> {
    match action {
        UtilAction::FromRlp(args) => print_json(
            output_mode,
            rlp::from_rlp(&structured_input(args.value, "from-rlp")?, args.as_int)?,
        ),
        UtilAction::ToRlp(args) => print_value(
            output_mode,
            rlp::to_rlp(&structured_input(args.value, "to-rlp")?)?,
            serde_json::json!({}),
        ),
        _ => unreachable!("unexpected util action for rlp family"),
    }
}
