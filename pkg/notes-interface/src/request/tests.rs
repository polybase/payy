use super::{NoteRequestInput, NoteRequestKind};

#[test]
fn note_request_input_ignores_removed_note_kind_field() {
    let input = serde_json::from_value::<NoteRequestInput>(serde_json::json!({
        "id": "ramp-id",
        "kind": NoteRequestKind::RampDeposit,
        "note_kind": element::Element::new(1),
    }))
    .unwrap();

    assert_eq!(input.id, "ramp-id");
    assert!(matches!(input.kind, NoteRequestKind::RampDeposit));
}
