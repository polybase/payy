use std::collections::HashMap;
#[cfg(feature = "diesel")]
use std::io::Write;

#[cfg(feature = "diesel")]
use diesel::{
    AsExpression,
    deserialize::{self, FromSql, FromSqlRow},
    pg::{Pg, PgValue},
    serialize::{self, IsNull, Output, ToSql},
    sql_types::Jsonb,
};
use serde::{Deserialize, Serialize};
#[cfg(feature = "ts-rs")]
use ts_rs::TS;

use crate::KycField;

#[derive(Default, Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE", tag = "reason", content = "data")]
#[cfg_attr(feature = "ts-rs", derive(TS))]
#[cfg_attr(feature = "ts-rs", ts(export))]
pub enum KycUpdateRequiredInvalidFieldsReason {
    #[default]
    Unspecified,
    NameMismatch,
    DocumentKycMismatch,
    DocumentPhotoBlurry,
    DocumentPhotoNotCentered,
    DocumentPhotoDamaged,
    DocumentPhotoNotOriginal,
    DocumentExpiresSoon,
    DocumentIsExpired,
    DocumentKindNotAllowed,
    AdverseMedia,
    AgeRequirementMismatch,
    Blacklist,
    Blocklist,
    CheckUnavailable,
    CompromisedPersons,
    Criminal,
    DbDataMismatch,
    DbDataNotFound,
    DocumentTemplate,
    Duplicate,
    ExperienceRequirementMismatch,
    Forgery,
    FraudulentLiveness,
    FraudulentPatterns,
    InconsistentProfile,
    Pep,
    RegulationsViolations,
    Sanctions,
    SelfieMismatch,
    Spam,
    NotDocument,
    ThirdPartyInvolved,
    UnsupportedLanguage,
    WrongUserRegion,
    InvalidPhoneNumber,
    InvalidEmail,
    InvalidFormat,
    InvalidPostalCode,
    InvalidNationalId,
    NameTooLong,
    AddressStreetTooLong,
    AddressCityTooLong,
    AddressStateTooLong,
    Compliance,
    Sanctioned,
    PoliticallyExposedPerson,
}

#[derive(Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
#[cfg_attr(feature = "ts-rs", derive(TS))]
#[cfg_attr(feature = "ts-rs", ts(export))]
pub struct KycUpdateRequiredInvalidFields {
    pub fields: HashMap<KycField, KycUpdateRequiredInvalidFieldsReason>,
}

#[derive(Default, Debug, Serialize, Deserialize, Clone, Eq, PartialEq)]
#[cfg_attr(feature = "diesel", derive(AsExpression, FromSqlRow))]
#[cfg_attr(feature = "diesel", diesel(sql_type = diesel::sql_types::Jsonb))]
#[serde(rename_all = "SCREAMING_SNAKE_CASE", tag = "error", content = "data")]
#[cfg_attr(feature = "ts-rs", derive(TS))]
#[cfg_attr(feature = "ts-rs", ts(export))]
pub enum KycUpdateRequired {
    #[default]
    None,
    UnsupportedCountry,
    // String should be an error message to be displayed to the user
    InvalidFields(KycUpdateRequiredInvalidFields),
    UserRequestedDelete,
    PayyComplianceViolation,
    Duplicate,
    AgeRestriction,
    DocumentForgery,
    InvalidDocumentPhoto,
    InconsistentDocuments,
}

impl KycUpdateRequired {
    pub fn merge(&self, other: &KycUpdateRequired) -> KycUpdateRequired {
        match (self, other) {
            (KycUpdateRequired::InvalidFields(a), KycUpdateRequired::InvalidFields(b)) => {
                let mut merged_fields = a.fields.clone();
                for (field, reason) in &b.fields {
                    merged_fields.insert(*field, *reason);
                }
                KycUpdateRequired::InvalidFields(KycUpdateRequiredInvalidFields {
                    fields: merged_fields,
                })
            }
            _ => other.clone(),
        }
    }
}

#[cfg(feature = "diesel")]
impl ToSql<Jsonb, Pg> for KycUpdateRequired {
    fn to_sql(&self, out: &mut Output<Pg>) -> serialize::Result {
        out.write_all(&[1])?;
        serde_json::to_writer(out, self)?;
        Ok(IsNull::No)
    }
}

#[cfg(feature = "diesel")]
impl FromSql<Jsonb, Pg> for KycUpdateRequired {
    fn from_sql(bytes: PgValue) -> deserialize::Result<Self> {
        let bytes = bytes.as_bytes();
        if bytes.is_empty() {
            return Ok(KycUpdateRequired::default());
        }
        let json_bytes = &bytes[1..];
        serde_json::from_slice(json_bytes).map_err(Into::into)
    }
}
