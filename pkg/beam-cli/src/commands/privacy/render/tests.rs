use contracts::U256;

use super::format_private_balance_value;

#[test]
fn private_balance_formats_decimal_atomic_values() {
    let value = U256::from_dec_str("1000000000000000000").expect("parse decimal atomic value");

    assert_eq!(format_private_balance_value(value, 18), "1");
}
