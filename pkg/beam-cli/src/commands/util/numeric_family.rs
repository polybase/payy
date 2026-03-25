use serde_json::json;

use crate::{cli::util::UtilAction, error::Result, output::OutputMode, util::numbers};

use super::render::{print_value, required_field, structured_input};

pub(super) fn run(output_mode: OutputMode, action: UtilAction) -> Result<()> {
    match action {
        UtilAction::FormatUnits(args) => print_value(
            output_mode,
            numbers::format_units_value(
                &structured_input(args.value, "format-units")?,
                args.unit.as_deref().unwrap_or("18"),
            )?,
            json!({}),
        ),
        UtilAction::FromFixedPoint(args) => print_value(
            output_mode,
            numbers::from_fixed_point(
                &required_field(args.decimals, "from-fixed-point <decimals>")?,
                &required_field(args.value, "from-fixed-point <value>")?,
            )?,
            json!({}),
        ),
        UtilAction::FromWei(args) => print_value(
            output_mode,
            numbers::from_wei(
                &structured_input(args.value, "from-wei")?,
                args.unit.as_deref(),
            )?,
            json!({}),
        ),
        UtilAction::MaxInt(args) => print_value(
            output_mode,
            numbers::max_int(args.ty.as_deref())?,
            json!({}),
        ),
        UtilAction::MaxUint(args) => print_value(
            output_mode,
            numbers::max_uint(args.ty.as_deref())?,
            json!({}),
        ),
        UtilAction::MinInt(args) => print_value(
            output_mode,
            numbers::min_int(args.ty.as_deref())?,
            json!({}),
        ),
        UtilAction::ParseUnits(args) => print_value(
            output_mode,
            numbers::parse_units_value(
                &structured_input(args.value, "parse-units")?,
                args.unit.as_deref().unwrap_or("18"),
            )?,
            json!({}),
        ),
        UtilAction::Shl(args) => print_value(
            output_mode,
            numbers::shift_left(
                &args.value,
                &args.bits,
                args.base_in.as_deref(),
                args.base_out.as_deref(),
            )?,
            json!({}),
        ),
        UtilAction::Shr(args) => print_value(
            output_mode,
            numbers::shift_right(
                &args.value,
                &args.bits,
                args.base_in.as_deref(),
                args.base_out.as_deref(),
            )?,
            json!({}),
        ),
        UtilAction::ToBase(args) => print_value(
            output_mode,
            numbers::to_base(
                &structured_input(args.value, "to-base")?,
                args.base_in.as_deref(),
                &required_field(args.base, "to-base <base>")?,
            )?,
            json!({}),
        ),
        UtilAction::ToDec(args) => print_value(
            output_mode,
            numbers::to_dec(
                &structured_input(args.value, "to-dec")?,
                args.base_in.as_deref(),
            )?,
            json!({}),
        ),
        UtilAction::ToFixedPoint(args) => print_value(
            output_mode,
            numbers::to_fixed_point(
                &required_field(args.decimals, "to-fixed-point <decimals>")?,
                &required_field(args.value, "to-fixed-point <value>")?,
            )?,
            json!({}),
        ),
        UtilAction::ToHex(args) => print_value(
            output_mode,
            numbers::to_hex(
                &structured_input(args.value, "to-hex")?,
                args.base_in.as_deref(),
            )?,
            json!({}),
        ),
        UtilAction::ToInt256(args) => print_value(
            output_mode,
            numbers::to_int256(&structured_input(args.value, "to-int256")?)?,
            json!({}),
        ),
        UtilAction::ToUint256(args) => print_value(
            output_mode,
            numbers::to_uint256(&structured_input(args.value, "to-uint256")?)?,
            json!({}),
        ),
        UtilAction::ToUnit(args) => print_value(
            output_mode,
            numbers::to_unit(
                &structured_input(args.value, "to-unit")?,
                args.unit.as_deref(),
            )?,
            json!({}),
        ),
        UtilAction::ToWei(args) => print_value(
            output_mode,
            numbers::to_wei(
                &structured_input(args.value, "to-wei")?,
                args.unit.as_deref(),
            )?,
            json!({}),
        ),
        _ => unreachable!("unexpected util action for numeric family"),
    }
}
