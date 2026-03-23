use super::create::CreateNoteInput;
use element::Element;
use zk_primitives::bridged_polygon_usdc_note_kind;

#[test]
fn create_note_input_defaults_note_kind_to_usdc() {
    let input = serde_json::from_value::<CreateNoteInput>(serde_json::json!({
        "private_key": Element::new(1),
        "psi": Element::new(2),
        "value": Element::new(3),
    }))
    .unwrap();

    assert_eq!(input.note_kind, bridged_polygon_usdc_note_kind());
}
