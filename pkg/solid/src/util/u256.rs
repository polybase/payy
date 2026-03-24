#![allow(clippy::assign_op_pattern)]
#![expect(clippy::manual_div_ceil)]

use uint::*;

construct_uint! {
    /// 256-bit unsigned integer.
    pub struct U256(4);
}
