use chrono::{DateTime, Utc};
use serde::{Deserializer, Serializer};

#[expect(clippy::ref_option)]
pub fn serialize_datetime_opt<S>(
    dt: &Option<DateTime<Utc>>,
    serializer: S,
) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match dt {
        Some(dt) => serializer.serialize_i64(dt.timestamp_micros()),
        None => serializer.serialize_none(),
    }
}

pub fn deserialize_datetime_opt<'de, D>(deserializer: D) -> Result<Option<DateTime<Utc>>, D::Error>
where
    D: Deserializer<'de>,
{
    struct OptionalDateTimeVisitor;

    impl serde::de::Visitor<'_> for OptionalDateTimeVisitor {
        type Value = Option<DateTime<Utc>>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a microsecond timestamp, ISO 8601 datetime string, or null")
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(None)
        }

        fn visit_unit<E>(self) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            Ok(None)
        }

        fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            DateTime::from_timestamp_micros(value)
                .map(Some)
                .ok_or_else(|| E::custom(format!("invalid timestamp: {value}")))
        }

        fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            if value > i64::MAX as u64 {
                return Err(E::custom(format!("timestamp too large: {value}")));
            }
            let value = i64::try_from(value)
                .map_err(|_| E::custom(format!("timestamp too large: {value}")))?;

            self.visit_i64(value)
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: serde::de::Error,
        {
            // Try parsing as ISO 8601 first
            if let Ok(dt) = DateTime::parse_from_rfc3339(value) {
                return Ok(Some(dt.with_timezone(&Utc)));
            }

            // Then try parsing as microsecond timestamp
            if let Ok(ts) = value.parse::<i64>()
                && let Some(dt) = DateTime::from_timestamp_micros(ts)
            {
                return Ok(Some(dt));
            }

            Err(E::custom(format!("invalid datetime format: {value}")))
        }
    }

    deserializer.deserialize_any(OptionalDateTimeVisitor)
}
