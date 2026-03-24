// lint-long-file-override allow-max-lines=500
use base64::Engine;
use chrono::NaiveDate;
use currency::{Country, CountryList};
use kyc::*;
use serde_json::json;
use std::collections::HashMap;

fn make_id_doc_front_bytes() -> IDDocument {
    IDDocument {
        front: Some(IDDocumentField::Bytes(vec![1, 2, 3])),
        back: None,
    }
}

fn make_id_doc_front_id_back_bytes() -> IDDocument {
    IDDocument {
        front: Some(IDDocumentField::Id {
            id: "file;abc".to_string(),
            bytes: vec![9, 9],
        }),
        back: Some(IDDocumentField::Bytes(vec![4, 5, 6])),
    }
}

fn make_id_doc_front_id_back_id() -> IDDocument {
    IDDocument {
        front: Some(IDDocumentField::Id {
            id: "file;front".to_string(),
            bytes: vec![7],
        }),
        back: Some(IDDocumentField::Id {
            id: "file;back".to_string(),
            bytes: vec![8],
        }),
    }
}

fn base_kyc_all_fields() -> Kyc {
    let mut docs_by_kind = HashMap::new();
    docs_by_kind.insert(IDKind::Passport, make_id_doc_front_bytes());
    let mut documents = HashMap::new();
    documents.insert(Country::US, docs_by_kind);

    Kyc {
        firstname: Some("Alice".to_string()),
        middlename: Some("B".to_string()),
        lastname: Some("Carroll".to_string()),
        dob: Some(NaiveDate::from_ymd_opt(1990, 1, 1).unwrap()),
        occupation: Some("Engineer".to_string()),
        addressstreet: Some("123 Main St".to_string()),
        addresscity: Some("Metropolis".to_string()),
        addressstate: Some("CA".to_string()),
        addresscountry: Some(Country::US),
        addresspostalcode: Some("94105".to_string()),
        nationalities: Some(CountryList(vec![Country::US, Country::CA])),
        documents: Some(documents),
        phone: Some("+14155552671".to_string()),
        phoneverified: Some(true),
        email: Some("alice@example.com".to_string()),
        emailverified: Some(true),
        nationalid: Some("SSN123".to_string()),
        pep: Some(false),
        civilstate: Some("Single".to_string()),
        fatca: Some(false),
        uif: Some(false),
        selfiefront: Some(IDDocumentField::Bytes(vec![42])),
        selfieleft: Some(IDDocumentField::Id {
            id: "file;left".to_string(),
            bytes: vec![1],
        }),
        selfieright: Some(IDDocumentField::Id {
            id: "file;right".to_string(),
            bytes: vec![2],
        }),
    }
}

#[test]
fn test_getters_success() {
    let kyc = base_kyc_all_fields();
    assert_eq!(kyc.get_firstname().unwrap(), "Alice");
    assert_eq!(kyc.get_lastname().unwrap(), "Carroll");
    assert_eq!(
        kyc.get_dob().unwrap(),
        NaiveDate::from_ymd_opt(1990, 1, 1).unwrap()
    );
    assert_eq!(kyc.get_occupation().unwrap(), "Engineer");
    assert_eq!(kyc.get_addressstreet().unwrap(), "123 Main St");
    assert_eq!(kyc.get_addresscity().unwrap(), "Metropolis");
    assert_eq!(kyc.get_addressstate().unwrap(), "CA");
    assert_eq!(kyc.get_addresscountry().unwrap(), Country::US);
    assert_eq!(kyc.get_addresspostalcode().unwrap(), "94105");
    let nats = kyc.get_nationalities().unwrap();
    assert_eq!(nats.0, vec![Country::US, Country::CA]);
    assert_eq!(kyc.get_nationality_one().unwrap(), Country::US);
    let (cc, national) = kyc.get_phone_parts().unwrap();
    assert_eq!(cc, "1");
    assert_eq!(national, "4155552671");
    assert!(kyc.get_phone_verified().unwrap());
    assert_eq!(kyc.get_email().unwrap(), "alice@example.com");
    assert!(kyc.get_email_verified().unwrap());
    assert_eq!(kyc.get_nationalid().unwrap(), "SSN123");
    assert_eq!(kyc.get_civil_state().unwrap(), "Single");
    assert!(!kyc.get_fatca().unwrap());
    assert!(!kyc.get_uif().unwrap());
    assert!(!kyc.get_pep().unwrap());
}

#[test]
fn test_getters_errors() {
    let mut kyc = Kyc {
        nationalities: Some(CountryList(vec![])),
        ..Default::default()
    };
    // nationalities empty triggers specific error in get_nationalities
    assert!(matches!(
        kyc.get_nationalities(),
        Err(Error::MissingKYCField(_))
    ));
    // nationality one when option is None
    kyc.nationalities = None;
    assert!(matches!(
        kyc.get_nationality_one(),
        Err(Error::MissingKYCField(_))
    ));
    // phone parts invalid
    kyc.phone = Some("abc".to_string());
    assert!(matches!(
        kyc.get_phone_parts(),
        Err(Error::InvalidPhoneFormat)
    ));
    // documents None -> errors
    assert!(matches!(
        kyc.get_document_by_country(Country::US),
        Err(Error::MissingKYCField(_))
    ));
    assert!(matches!(
        kyc.get_documents_by_country_and_kind(Country::US, IDKind::Passport),
        Err(Error::MissingKYCField(_))
    ));
    assert!(matches!(
        kyc.get_documents_countries(),
        Err(Error::MissingKYCField(_))
    ));
}

#[test]
fn test_documents_queries() {
    let mut kyc = base_kyc_all_fields();
    // add another kind to test preference order
    if let Some(ref mut docs) = kyc.documents {
        let entry = docs.entry(Country::US).or_insert_with(HashMap::new);
        entry.insert(IDKind::DriversLicense, make_id_doc_front_id_back_id());
    }
    // get by country returns some pair
    let (kind, doc) = kyc.get_document_by_country(Country::US).unwrap();
    assert!(matches!(kind, IDKind::Passport | IDKind::DriversLicense));
    assert!(doc.front.is_some() || doc.back.is_some());
    // get by country and kind
    let dl = kyc
        .get_documents_by_country_and_kind(Country::US, IDKind::DriversLicense)
        .unwrap();
    assert!(dl.front.is_some());
    // one of kinds prefers first present
    let chosen = kyc
        .get_one_of_document_kinds(Country::US, &[IDKind::ResidentCard, IDKind::Passport])
        .unwrap();
    assert!(chosen.front.is_some());
    // missing country/kind errors
    assert!(matches!(
        kyc.get_documents_by_country_and_kind(Country::CA, IDKind::Passport),
        Err(Error::MissingKYCField(_))
    ));
    assert!(matches!(
        kyc.get_one_of_document_kinds(Country::US, &[IDKind::ResidentCard, IDKind::StateId]),
        Err(Error::MissingKYCField(_))
    ));
    // countries list and first
    let countries = kyc.get_documents_countries().unwrap();
    assert!(countries.contains(&Country::US));
    let one = kyc.get_documents_countries_one().unwrap();
    assert!(one == Country::US);
    // empty map -> countries returns empty and countries_one errors
    let kyc_empty = Kyc {
        documents: Some(HashMap::new()),
        ..Default::default()
    };
    assert_eq!(
        kyc_empty.get_documents_countries().unwrap(),
        Vec::<Country>::new()
    );
    assert!(matches!(
        kyc_empty.get_documents_countries_one(),
        Err(Error::MissingKYCField(_))
    ));
}

#[test]
fn test_invalid_field_reason_name_too_long_serializes_as_expected() {
    let value = serde_json::to_value(KycUpdateRequiredInvalidFieldsReason::NameTooLong).unwrap();
    assert_eq!(value, json!({ "reason": "NAME_TOO_LONG" }));
}

#[test]
fn test_merge_prefers_new_and_sanitizes_new_documents() {
    let base = base_kyc_all_fields();
    let mut new_docs_by_kind = HashMap::new();
    new_docs_by_kind.insert(IDKind::DriversLicense, make_id_doc_front_id_back_bytes());
    let mut new_documents = HashMap::new();
    new_documents.insert(Country::US, new_docs_by_kind);
    // Also include a country with doc that should not be sanitized (both fields are Id)
    let mut ca_docs = HashMap::new();
    ca_docs.insert(IDKind::StateId, make_id_doc_front_id_back_id());
    new_documents.insert(Country::CA, ca_docs);
    let update = Kyc {
        firstname: Some("Alicia".to_string()),
        middlename: None,
        lastname: Some("Carroll-Updated".to_string()),
        dob: None,
        occupation: Some("Senior Engineer".to_string()),
        addressstreet: Some("456 Market St".to_string()),
        addresscity: None,
        addressstate: Some("California".to_string()),
        addresscountry: None,
        addresspostalcode: Some("94107".to_string()),
        nationalities: None,
        documents: Some(new_documents),
        phone: Some("0014155552671".to_string()),
        phoneverified: Some(false),
        email: None,
        emailverified: Some(false),
        nationalid: None,
        pep: Some(true),
        civilstate: Some("Married".to_string()),
        fatca: Some(true),
        uif: Some(true),
        selfiefront: Some(IDDocumentField::Id {
            id: "file;selfie".to_string(),
            bytes: vec![],
        }),
        selfieleft: None,
        selfieright: None,
    };

    let merged = base.merge(&update);
    // Fields prefer update when Some
    assert_eq!(merged.firstname.as_deref(), Some("Alicia"));
    assert_eq!(merged.lastname.as_deref(), Some("Carroll-Updated"));
    assert_eq!(merged.occupation.as_deref(), Some("Senior Engineer"));
    assert_eq!(merged.addressstreet.as_deref(), Some("456 Market St"));
    assert_eq!(merged.addressstate.as_deref(), Some("California"));
    assert_eq!(merged.addresspostalcode.as_deref(), Some("94107"));
    assert_eq!(merged.phone.as_deref(), Some("0014155552671"));
    assert_eq!(merged.phoneverified, Some(false));
    assert_eq!(merged.email.as_deref(), Some("alice@example.com")); // fell back to base
    assert_eq!(merged.emailverified, Some(false));
    assert_eq!(merged.nationalid.as_deref(), Some("SSN123")); // fell back to base
    assert_eq!(merged.pep, Some(true));
    assert_eq!(merged.civilstate.as_deref(), Some("Married"));
    assert_eq!(merged.fatca, Some(true));
    assert_eq!(merged.uif, Some(true));
    // Fallbacks for None in update
    assert_eq!(merged.middlename.as_deref(), Some("B"));
    assert_eq!(merged.addresscity.as_deref(), Some("Metropolis"));
    assert_eq!(
        merged.dob,
        Some(NaiveDate::from_ymd_opt(1990, 1, 1).unwrap())
    );
    assert_eq!(merged.addresscountry, Some(Country::US));
    assert_eq!(
        merged.nationalities.as_ref().map(|n| &n.0[..]),
        Some(&[Country::US, Country::CA][..])
    );
    // Selfies: prefer update when present; left/right fallback
    match merged.selfiefront {
        Some(IDDocumentField::Id { ref id, .. }) => assert_eq!(id, "file;selfie"),
        _ => panic!("expected selfiefront from update"),
    }
    match merged.selfieleft {
        Some(IDDocumentField::Id { ref id, .. }) => assert_eq!(id, "file;left"),
        _ => panic!("expected selfieleft from base"),
    }
    match merged.selfieright {
        Some(IDDocumentField::Id { ref id, .. }) => assert_eq!(id, "file;right"),
        _ => panic!("expected selfieright from base"),
    }
    // Documents: base passport remains, new US DL is sanitized (no bytes retained), CA doc remains intact
    let docs = merged.documents.unwrap();
    let us_docs = docs.get(&Country::US).unwrap();
    assert!(us_docs.contains_key(&IDKind::Passport));
    let dl = us_docs.get(&IDKind::DriversLicense).unwrap();
    assert!(dl.front.is_none() && dl.back.is_none());
    let ca_docs = docs.get(&Country::CA).unwrap();
    let ca_doc = ca_docs.get(&IDKind::StateId).unwrap();
    assert!(ca_doc.front.is_some() && ca_doc.back.is_some());
}

#[test]
fn test_merge_update_without_documents_keeps_base_documents() {
    let base = base_kyc_all_fields();
    let update = Kyc {
        documents: None,
        ..Default::default()
    };
    let merged = base.merge(&update);
    let base_docs = base.documents.unwrap();
    let merged_docs = merged.documents.unwrap();
    assert_eq!(
        merged_docs.keys().cloned().collect::<Vec<_>>(),
        base_docs.keys().cloned().collect::<Vec<_>>()
    );
    let us_docs_base = base_docs.get(&Country::US).unwrap();
    let us_docs_merged = merged_docs.get(&Country::US).unwrap();
    assert!(us_docs_merged.contains_key(&IDKind::Passport));
    assert_eq!(us_docs_merged.len(), us_docs_base.len());
}

#[test]
fn test_sanitize_for_storage_no_documents_is_noop() {
    let kyc = Kyc {
        firstname: Some("Alice".to_string()),
        ..Default::default()
    };
    let sanitized = kyc.sanitize_for_storage();
    assert_eq!(sanitized.firstname.as_deref(), Some("Alice"));
    assert!(sanitized.documents.is_none());
}

#[test]
fn test_selfies_serde_behavior() {
    let kyc = Kyc {
        selfiefront: Some(IDDocumentField::Bytes(vec![1, 2])),
        selfieleft: Some(IDDocumentField::Id {
            id: "file;left".to_string(),
            bytes: vec![9],
        }),
        selfieright: None,
        ..Default::default()
    };
    let value = serde_json::to_value(&kyc).unwrap();
    let obj = value.as_object().unwrap();
    // Bytes variant serializes as base64 string
    let selfiefront_val = obj.get("selfiefront").unwrap();
    assert_eq!(
        selfiefront_val.as_str().unwrap(),
        base64::engine::general_purpose::STANDARD.encode([1u8, 2u8])
    );
    // Id variant serializes as the id string
    let selfieleft_val = obj.get("selfieleft").unwrap();
    assert_eq!(selfieleft_val.as_str().unwrap(), "file;left");
    // None is skipped
    assert!(!obj.contains_key("selfieright"));
    // Roundtrip deserialize
    let round: Kyc = serde_json::from_value(value).unwrap();
    match round.selfiefront.unwrap() {
        IDDocumentField::Bytes(bytes) => assert_eq!(bytes, vec![1, 2]),
        _ => panic!("expected selfiefront bytes after roundtrip"),
    }
    match round.selfieleft.unwrap() {
        IDDocumentField::Id { id, .. } => assert_eq!(id, "file;left"),
        _ => panic!("expected selfieleft id after roundtrip"),
    }
}

#[test]
fn test_sanitize_for_storage_removes_bytes_keeps_ids() {
    let mut docs_by_kind = HashMap::new();
    docs_by_kind.insert(
        IDKind::Passport,
        IDDocument {
            front: Some(IDDocumentField::Bytes(vec![1])),
            back: Some(IDDocumentField::Id {
                id: "file;keep".to_string(),
                bytes: vec![2],
            }),
        },
    );
    docs_by_kind.insert(
        IDKind::StateId,
        IDDocument {
            front: Some(IDDocumentField::Id {
                id: "file;front".to_string(),
                bytes: vec![3],
            }),
            back: Some(IDDocumentField::Bytes(vec![4])),
        },
    );
    let mut documents = HashMap::new();
    documents.insert(Country::US, docs_by_kind);
    let kyc = Kyc {
        documents: Some(documents),
        ..Default::default()
    };
    let sanitized = kyc.sanitize_for_storage();
    let docs = sanitized.documents.unwrap();
    let us_docs = docs.get(&Country::US).unwrap();
    let passport = us_docs.get(&IDKind::Passport).unwrap();
    assert!(passport.front.is_none());
    match passport.back {
        Some(IDDocumentField::Id { ref id, .. }) => assert_eq!(id, "file;keep"),
        _ => panic!("expected back id to be kept"),
    }
    let state_id = us_docs.get(&IDKind::StateId).unwrap();
    match state_id.front {
        Some(IDDocumentField::Id { ref id, .. }) => assert_eq!(id, "file;front"),
        _ => panic!("expected front id to be kept"),
    }
    assert!(state_id.back.is_none());
}

#[test]
fn test_phone_parts_handles_leading_zeros_and_no_plus() {
    let kyc = Kyc {
        phone: Some("0014155552671".to_string()),
        ..Default::default()
    };
    let (cc, national) = kyc.get_phone_parts().unwrap();
    assert_eq!(cc, "1");
    assert_eq!(national, "4155552671");
}

#[test]
fn test_all_other_getters_missing_field_errors() {
    let kyc = Kyc::default();
    assert!(matches!(
        kyc.get_firstname(),
        Err(Error::MissingKYCField(_))
    ));
    assert!(matches!(kyc.get_lastname(), Err(Error::MissingKYCField(_))));
    assert!(matches!(kyc.get_dob(), Err(Error::MissingKYCField(_))));
    assert!(matches!(
        kyc.get_occupation(),
        Err(Error::MissingKYCField(_))
    ));
    assert!(matches!(
        kyc.get_addressstreet(),
        Err(Error::MissingKYCField(_))
    ));
    assert!(matches!(
        kyc.get_addresscity(),
        Err(Error::MissingKYCField(_))
    ));
    assert!(matches!(
        kyc.get_addressstate(),
        Err(Error::MissingKYCField(_))
    ));
    assert!(matches!(
        kyc.get_addresscountry(),
        Err(Error::MissingKYCField(_))
    ));
    assert!(matches!(
        kyc.get_addresspostalcode(),
        Err(Error::MissingKYCField(_))
    ));
    assert!(matches!(kyc.get_phone(), Err(Error::MissingKYCField(_))));
    assert!(matches!(
        kyc.get_phone_verified(),
        Err(Error::MissingKYCField(_))
    ));
    assert!(matches!(kyc.get_email(), Err(Error::MissingKYCField(_))));
    assert!(matches!(
        kyc.get_email_verified(),
        Err(Error::MissingKYCField(_))
    ));
    assert!(matches!(
        kyc.get_nationalid(),
        Err(Error::MissingKYCField(_))
    ));
    assert!(matches!(
        kyc.get_civil_state(),
        Err(Error::MissingKYCField(_))
    ));
    assert!(matches!(kyc.get_fatca(), Err(Error::MissingKYCField(_))));
    assert!(matches!(kyc.get_uif(), Err(Error::MissingKYCField(_))));
    assert!(matches!(kyc.get_pep(), Err(Error::MissingKYCField(_))));
    // get_phone_parts should also error when phone is missing
    assert!(matches!(
        kyc.get_phone_parts(),
        Err(Error::MissingKYCField(_))
    ));
}
