use super::Element;
use crate::U256;
use diesel::{
    deserialize::{self, FromSql},
    pg::{Pg, PgValue},
    serialize::{self, IsNull, Output, ToSql},
    sql_types::Numeric,
};
use std::io::Write;
use std::{convert::TryInto, str::FromStr};

impl ToSql<Numeric, Pg> for Element {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        // Convert U256 to decimal string first
        let decimal_str = self.0.to_string();

        // Convert to base-10000 digits
        let mut digits = Vec::new();
        let mut remaining = decimal_str;
        while remaining.len() > 4 {
            let (rest, chunk) = remaining.split_at(remaining.len() - 4);
            digits.push(
                chunk
                    .parse::<u16>()
                    .map_err(|_| "Invalid numeric conversion")?,
            );
            remaining = rest.to_string();
        }
        if !remaining.is_empty() {
            digits.push(
                remaining
                    .parse::<u16>()
                    .map_err(|_| "Invalid numeric conversion")?,
            );
        }
        digits.reverse();

        // Write header
        let digit_count: u16 = digits
            .len()
            .try_into()
            .map_err(|_| "Number too large for PostgreSQL numeric format")?;
        out.write_all(&digit_count.to_be_bytes())?; // Number of digits

        let weight = (digit_count) - 1; // Weight of first digit
        out.write_all(&weight.to_be_bytes())?;

        out.write_all(&[0x00, 0x00])?; // Sign: 0x0000 for positive
        out.write_all(&[0x00, 0x00])?; // Display scale: 0

        // Write digits in network byte order (big-endian)
        for digit in digits {
            out.write_all(&digit.to_be_bytes())?;
        }

        Ok(IsNull::No)
    }
}

impl FromSql<Numeric, Pg> for Element {
    fn from_sql(bytes: PgValue<'_>) -> deserialize::Result<Self> {
        let bytes = bytes.as_bytes();
        if bytes.len() < 8 {
            return Err("Invalid numeric format: too short".into());
        }

        // Read header
        let ndigits = u16::from_be_bytes([bytes[0], bytes[1]]) as usize;
        let weight = i16::from_be_bytes([bytes[2], bytes[3]]);
        let sign = u16::from_be_bytes([bytes[4], bytes[5]]);
        let _dscale = u16::from_be_bytes([bytes[6], bytes[7]]);

        if sign != 0x0000 {
            return Err("Expected positive number".into());
        }

        // Read digits
        let mut result = U256::ZERO;
        for i in 0..ndigits {
            let start = 8 + (i * 2);
            if start + 2 > bytes.len() {
                return Err("Invalid numeric format: truncated digits".into());
            }

            let digit = u16::from_be_bytes([bytes[start], bytes[start + 1]]);
            // Each PostgreSQL digit is base-10000
            result *= U256::from(10000u32);
            result += U256::from(digit);
        }

        // Adjust for weight
        // weight is the number of 4-digit groups before the decimal point minus 1
        let weight_adjustment = i32::from(weight)
            - i32::try_from(ndigits)
                .map_err(|_| "Number too large")?
                .checked_sub(1)
                .ok_or("Arithmetic overflow")?;
        if weight_adjustment > 0 {
            result *= U256::from(10000u32).pow(weight_adjustment.try_into().unwrap());
        }

        Ok(Element(result))
    }
}

impl ToSql<diesel::sql_types::Text, Pg> for Element {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> serialize::Result {
        let text = self.to_hex();
        out.write_all(text.as_bytes())?;
        Ok(IsNull::No)
    }
}

impl FromSql<diesel::sql_types::Text, Pg> for Element {
    fn from_sql(bytes: PgValue<'_>) -> deserialize::Result<Self> {
        let text = std::str::from_utf8(bytes.as_bytes())?;
        let value = Element::from_str(text)
            .map_err(|_| "Invalid decimal string for Element".to_string())?;
        Ok(value)
    }
}
