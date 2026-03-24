use super::InputNote;
use crate::note::Note;
use crate::{bridged_polygon_usdc_note_kind, get_address_for_private_key};
use element::Element;

#[test]
fn to_note_url_payload_omits_default_usdc_note_kind() {
    let note_kind = bridged_polygon_usdc_note_kind();
    let input_note = InputNote::new(
        Note::new_from_ephemeral_private_key(Element::new(10), Element::new(20), note_kind),
        Element::new(10),
    );

    let payload = input_note.to_note_url_payload();

    assert_eq!(payload.note_kind, None);
    assert_eq!(payload.note_kind(), note_kind);
}

#[test]
fn to_note_url_payload_preserves_non_usdc_note_kind() {
    let note_kind = Element::new(99);
    let secret_key = Element::new(10);
    let input_note = InputNote::new(
        Note::new_with_psi(
            get_address_for_private_key(secret_key),
            Element::new(20),
            Element::new(30),
            note_kind,
        ),
        secret_key,
    );

    let payload = input_note.to_note_url_payload();

    assert_eq!(payload.note_kind, Some(note_kind));
    assert_eq!(payload.note_kind(), note_kind);
}

#[test]
fn input_note_from_payload_uses_payload_note_kind() {
    let note_kind = Element::new(99);
    let payload = parse_link::NoteURLPayload {
        version: 2,
        private_key: Element::new(10),
        psi: None,
        value: Element::new(20),
        note_kind: Some(note_kind),
        referral_code: String::new(),
    };

    let input_note = InputNote::from(&payload);

    assert_eq!(input_note.note.contract, note_kind);
}

#[test]
fn generate_link_roundtrips_non_usdc_note_kind() {
    let note_kind = Element::new(99);
    let input_note = InputNote::new(
        Note::new_from_ephemeral_private_key(Element::new(10), Element::new(20), note_kind),
        Element::new(10),
    );

    let link = input_note.generate_link();
    let decoded = InputNote::new_from_link(&link);

    assert_eq!(decoded.note.contract, note_kind);
}
