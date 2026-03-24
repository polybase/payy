use super::{Error, Result};

pub fn abbreviate_state(state: &str) -> Result<&'static str> {
    let normalized = state.trim().to_uppercase();

    let abbreviation = match normalized.as_str() {
        "ALABAMA" | "AL" => Some("AL"),
        "ALASKA" | "AK" => Some("AK"),
        "ARIZONA" | "AZ" => Some("AZ"),
        "ARKANSAS" | "AR" => Some("AR"),
        "CALIFORNIA" | "CA" => Some("CA"),
        "COLORADO" | "CO" => Some("CO"),
        "CONNECTICUT" | "CT" => Some("CT"),
        "DELAWARE" | "DE" => Some("DE"),
        "FLORIDA" | "FL" => Some("FL"),
        "GEORGIA" | "GA" => Some("GA"),
        "HAWAII" | "HI" => Some("HI"),
        "IDAHO" | "ID" => Some("ID"),
        "ILLINOIS" | "IL" => Some("IL"),
        "INDIANA" | "IN" => Some("IN"),
        "IOWA" | "IA" => Some("IA"),
        "KANSAS" | "KS" => Some("KS"),
        "KENTUCKY" | "KY" => Some("KY"),
        "LOUISIANA" | "LA" => Some("LA"),
        "MAINE" | "ME" => Some("ME"),
        "MARYLAND" | "MD" => Some("MD"),
        "MASSACHUSETTS" | "MA" => Some("MA"),
        "MICHIGAN" | "MI" => Some("MI"),
        "MINNESOTA" | "MN" => Some("MN"),
        "MISSISSIPPI" | "MS" => Some("MS"),
        "MISSOURI" | "MO" => Some("MO"),
        "MONTANA" | "MT" => Some("MT"),
        "NEBRASKA" | "NE" => Some("NE"),
        "NEVADA" | "NV" => Some("NV"),
        "NEW HAMPSHIRE" | "NH" => Some("NH"),
        "NEW JERSEY" | "NJ" => Some("NJ"),
        "NEW MEXICO" | "NM" => Some("NM"),
        "NEW YORK" | "NY" => Some("NY"),
        "NORTH CAROLINA" | "NC" => Some("NC"),
        "NORTH DAKOTA" | "ND" => Some("ND"),
        "OHIO" | "OH" => Some("OH"),
        "OKLAHOMA" | "OK" => Some("OK"),
        "OREGON" | "OR" => Some("OR"),
        "PENNSYLVANIA" | "PA" => Some("PA"),
        "RHODE ISLAND" | "RI" => Some("RI"),
        "SOUTH CAROLINA" | "SC" => Some("SC"),
        "SOUTH DAKOTA" | "SD" => Some("SD"),
        "TENNESSEE" | "TN" => Some("TN"),
        "TEXAS" | "TX" => Some("TX"),
        "UTAH" | "UT" => Some("UT"),
        "VERMONT" | "VT" => Some("VT"),
        "VIRGINIA" | "VA" => Some("VA"),
        "WASHINGTON" | "WA" => Some("WA"),
        "WEST VIRGINIA" | "WV" => Some("WV"),
        "WISCONSIN" | "WI" => Some("WI"),
        "WYOMING" | "WY" => Some("WY"),
        "DISTRICT OF COLUMBIA" | "DC" => Some("DC"),
        _ => None,
    };

    if let Some(value) = abbreviation {
        Ok(value)
    } else {
        Err(Error::InvalidState { state: normalized })
    }
}

pub fn ok_field<T: Clone>(field: &Option<T>, field_name: &'static str) -> Result<T> {
    field
        .clone()
        .ok_or(Error::MissingKYCField(field_name.to_string()))
}

#[cfg(test)]
mod test {
    use super::{Error, abbreviate_state};

    #[test]
    fn test_abbreviate_state() {
        let state = abbreviate_state("ALABAMA").unwrap();

        assert_eq!(state, "AL")
    }

    #[test]
    fn test_abbreviate_state_invalid() {
        let error = abbreviate_state("Atlantis").unwrap_err();

        assert!(matches!(
            error,
            Error::InvalidState { state } if state == "ATLANTIS"
        ));

        let trimmed = abbreviate_state("  narnia ").unwrap_err();

        assert!(matches!(
            trimmed,
            Error::InvalidState { state } if state == "NARNIA"
        ));
    }
}
