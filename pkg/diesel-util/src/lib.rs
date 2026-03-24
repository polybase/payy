#![warn(clippy::pedantic)]
#![allow(missing_docs)]

#[cfg(feature = "diesel")]
#[macro_export]
macro_rules! derive_pg_text_enum {
    ($enum_name:ident, $case_style:expr) => {
        impl diesel::serialize::ToSql<diesel::sql_types::Text, diesel::pg::Pg> for $enum_name {
            fn to_sql(
                &self,
                out: &mut diesel::serialize::Output<diesel::pg::Pg>,
            ) -> diesel::serialize::Result {
                use std::io::Write;
                write!(out, "{self}")?;
                Ok(diesel::serialize::IsNull::No)
            }
        }

        impl diesel::deserialize::FromSql<diesel::sql_types::Text, diesel::pg::Pg> for $enum_name {
            fn from_sql(bytes: diesel::pg::PgValue) -> diesel::deserialize::Result<Self> {
                let s = std::str::from_utf8(bytes.as_bytes())?;
                s.parse().map_err(|_| "Unrecognized value".into())
            }
        }
    };

    ($enum_name:ident) => {
        $crate::derive_pg_text_enum!($enum_name, "SCREAMING_SNAKE_CASE")
    };
}
