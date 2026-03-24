use super::*;

const LEGACY_V2_USDC_PAYLOAD: &str = "9YcSG3egf5iEsLFFWTzHBhavV2WBka2HPYR5sx5giwYTTs8tJbDX6HwA4r";
const NON_USDC_V2_PAYLOAD: &str = "aMf8si8A6coYZqmSTSkp7mNdGVeAWgvZic1SgEdgUkUTbZigL3sVJPwbgXUdJstTVrwVRuskgUTDRgwHZDg1AbYUoGkxR9AJmAt2";

#[test]
fn encode_decode_roundtrip_version_2() {
    let payload = NoteURLPayload {
        version: 2,
        private_key: Element::new(101),
        psi: None,
        value: Element::new(1_000_000),
        note_kind: None,
        referral_code: "REF123".to_owned(),
    };

    let encoded = payload.encode_activity_url_payload();
    let decoded = decode_activity_url_payload(&encoded);

    assert_eq!(encoded, LEGACY_V2_USDC_PAYLOAD);
    assert_eq!(decoded.version, payload.version);
    assert_eq!(decoded.private_key, payload.private_key);
    assert_eq!(decoded.value, payload.value);
    assert_eq!(decoded.note_kind(), bridged_polygon_usdc_note_kind());
    assert_eq!(decoded.referral_code, payload.referral_code);
    assert_eq!(decoded.psi(), payload.psi());
}

#[test]
fn decode_old_links_defaults_to_usdc_note_kind() {
    let decoded = decode_activity_url_payload(LEGACY_V2_USDC_PAYLOAD);

    assert_eq!(decoded.note_kind, None);
    assert_eq!(decoded.note_kind(), bridged_polygon_usdc_note_kind());
}

#[test]
fn encode_decode_roundtrip_preserves_non_usdc_note_kind() {
    let note_kind = Element::new(99);
    let payload = NoteURLPayload {
        version: 2,
        private_key: Element::new(33),
        psi: None,
        value: Element::new(44),
        note_kind: Some(note_kind),
        referral_code: "REF123".to_owned(),
    };

    let encoded = payload.encode_activity_url_payload();
    let decoded = decode_activity_url_payload(&encoded);

    assert_eq!(encoded, NON_USDC_V2_PAYLOAD);
    assert_eq!(decoded.version, payload.version);
    assert_eq!(decoded.note_kind, Some(note_kind));
    assert_eq!(decoded.note_kind(), note_kind);
    assert_eq!(decoded.referral_code, payload.referral_code);
}

#[test]
fn encode_with_explicit_usdc_note_kind_matches_legacy_format() {
    let legacy_payload = NoteURLPayload {
        version: 2,
        private_key: Element::new(33),
        psi: None,
        value: Element::new(44),
        note_kind: None,
        referral_code: "REF123".to_owned(),
    };
    let explicit_usdc_payload = NoteURLPayload {
        note_kind: Some(bridged_polygon_usdc_note_kind()),
        ..legacy_payload.clone()
    };

    assert_eq!(
        explicit_usdc_payload.encode_activity_url_payload(),
        legacy_payload.encode_activity_url_payload()
    );
}

#[test]
fn try_decode_detects_invalid_payload() {
    let err = try_decode_activity_url_payload("invalid").unwrap_err();
    assert!(matches!(err, NoteUrlDecodeError::Base58(_)));
}

#[test]
fn psi_derivation_matches_versions() {
    let explicit_psi = Element::new(42);
    let payload_v0 = NoteURLPayload {
        version: 0,
        private_key: Element::new(7),
        psi: Some(explicit_psi),
        value: Element::new(10),
        note_kind: None,
        referral_code: String::new(),
    };
    assert_eq!(payload_v0.psi(), explicit_psi);

    let payload_v2 = NoteURLPayload {
        version: 2,
        private_key: Element::new(99),
        psi: None,
        value: Element::new(10),
        note_kind: None,
        referral_code: String::new(),
    };

    let derived = hash_private_key_for_psi(payload_v2.private_key);
    assert_eq!(payload_v2.psi(), derived);
}

#[test]
fn address_and_commitment_helpers_work() {
    let payload = NoteURLPayload {
        version: 2,
        private_key: Element::new(11),
        psi: None,
        value: Element::new(5),
        note_kind: Some(Element::new(99)),
        referral_code: String::new(),
    };

    let address = payload.address();
    assert_eq!(address, get_address_for_private_key(payload.private_key));

    let commitment = payload.commitment();
    let expected = hash_merge([
        Element::new(2),
        payload.note_kind(),
        payload.value,
        address,
        payload.psi(),
        Element::ZERO,
        Element::ZERO,
    ]);
    assert_eq!(commitment, expected);
}
