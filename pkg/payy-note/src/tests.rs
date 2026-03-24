use crate::{NoteGuard, NoteSelector, NoteValue};
use element::Element;
use zk_primitives::Note;

fn note_with_kind(kind: Element) -> Note {
    Note {
        kind: Element::new(2),
        contract: kind,
        address: Element::from(11u64),
        psi: Element::from(22u64),
        value: Element::from(33u64),
    }
}

#[test]
fn select_accepts_matching_kind() {
    let note_kind = Element::from(777u64);
    let note = note_with_kind(note_kind);

    let selected_note = NoteGuard::new(note.clone()).select(NoteSelector::new(note_kind));

    let Ok(selected_note) = selected_note else {
        panic!("expected note to match selector");
    };
    assert_eq!(selected_note.as_note(), &note);
}

#[test]
fn select_rejects_mismatched_kind() {
    let note_kind = Element::from(777u64);
    let other_kind = Element::from(888u64);
    let note = note_with_kind(note_kind);

    let selected_note = NoteGuard::new(note).select(NoteSelector::new(other_kind));

    let Err(note_guard) = selected_note else {
        panic!("expected note mismatch");
    };
    let selected_note = note_guard.select(NoteSelector::new(note_kind));
    let Ok(selected_note) = selected_note else {
        panic!("expected note to remain available for matching selector");
    };
    assert_eq!(selected_note.as_note().contract, note_kind);
}

#[test]
fn selected_note_into_note_returns_note() {
    let note_kind = Element::from(777u64);
    let note = note_with_kind(note_kind);
    let selected_note = NoteGuard::new(note.clone()).select(NoteSelector::new(note_kind));
    let Ok(selected_note) = selected_note else {
        panic!("note should match selector");
    };

    assert_eq!(selected_note.into_note(), note);
}

#[test]
fn matches_respects_selector_note_kind() {
    let note_kind = Element::from(777u64);
    let other_kind = Element::from(888u64);
    let note_guard = NoteGuard::new(note_with_kind(note_kind));

    assert!(note_guard.matches(NoteSelector::new(note_kind)));
    assert!(!note_guard.matches(NoteSelector::new(other_kind)));
}

#[test]
fn note_selector_roundtrip_preserves_kind() {
    let selector = NoteSelector::new(Element::from(555u64));

    let value = serde_json::to_value(selector).expect("serialize selector");
    let parsed: NoteSelector = serde_json::from_value(value).expect("deserialize selector");

    assert_eq!(parsed, selector);
}

#[test]
fn note_value_roundtrip_preserves_fields() {
    let note_value = NoteValue {
        value: Element::from(12u64),
        note_kind: Element::from(34u64),
    };

    let value = serde_json::to_value(note_value).expect("serialize note value");
    let parsed: NoteValue = serde_json::from_value(value).expect("deserialize note value");

    assert_eq!(parsed, note_value);
    assert_eq!(parsed.value, Element::from(12u64));
    assert_eq!(parsed.note_kind, Element::from(34u64));
}
