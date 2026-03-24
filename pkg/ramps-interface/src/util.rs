//! Miscellaneous helpers shared across ramps interface modules.

use chrono::Utc;
use rand::Rng;

/// Generates a unique, human-readable identifier for local method records.
///
/// The identifier embeds a timestamp to preserve ordering while avoiding
/// collisions via a random suffix.
#[must_use]
pub fn local_uid() -> String {
    let now = Utc::now().timestamp_millis();
    let random = rand::thread_rng().gen_range(0..1_000_000_000);
    format!("id-{now}-{random}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generates_distinct_values() {
        let uid1 = local_uid();
        let uid2 = local_uid();

        assert!(uid1.starts_with("id-"));
        assert!(uid2.starts_with("id-"));
        assert_ne!(uid1, uid2);

        let parts: Vec<&str> = uid1.split('-').collect();
        assert_eq!(parts.len(), 3);
        assert!(parts[1].parse::<i64>().is_ok());
        assert!(parts[2].parse::<i64>().is_ok());
    }
}
