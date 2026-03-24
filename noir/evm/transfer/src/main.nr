use common::{get_note_commitment, InputNote, MerklePath, Note, Utxo};
use poseidon::poseidon2;

fn main(
    utxo: Utxo,
    recent_root: pub Field,
    input_nullifiers: pub [Field; 2],
    output_commitments: pub [Field; 2],
) {
    let input_value = utxo.input_notes[0].note.value + utxo.input_notes[1].note.value;
    let output_value = utxo.output_notes[0].value + utxo.output_notes[1].value;
    assert(input_value == output_value, "Input and output totals do not match");

    utxo.validate(recent_root, input_nullifiers, output_commitments);
}

#[test]
fn test_transfer_one_input_one_output() {
    let pk: Field = 101;
    let address = common::get_address(pk);

    let note_1 = Note { kind: 1, value: 10, address, psi: 1 };
    let note_2 = Note { kind: 0, value: 0, address: 0, psi: 0 };
    let input_note_1 = InputNote { note: note_1, secret_key: pk };
    let input_note_2 = InputNote { note: note_2, secret_key: 0 };

    let note_3 = Note { kind: 1, value: 10, address, psi: 3 };
    let note_4 = Note { kind: 0, value: 0, address: 0, psi: 0 };

    let input_commitment_1 = get_note_commitment(note_1);
    let output_commitment_1 = get_note_commitment(note_3);
    let output_commitment_2 = get_note_commitment(note_4);

    let input_path_1 = MerklePath { path: [0; 160] };
    let input_path_2 = MerklePath { path: [0; 160] };
    let recent_root = input_path_1.root_from_leaf(input_commitment_1);
    let utxo = Utxo {
        input_notes: [input_note_1, input_note_2],
        output_notes: [note_3, note_4],
        input_merkle_paths: [input_path_1, input_path_2],
    };

    main(
        utxo,
        recent_root,
        [poseidon2::Poseidon2::hash([input_commitment_1, note_1.psi], 2), 0],
        [output_commitment_1, output_commitment_2],
    );
}

#[test(should_fail)]
fn test_transfer_rejects_bad_recent_root() {
    let pk: Field = 101;
    let address = common::get_address(pk);

    let note_1 = Note { kind: 1, value: 10, address, psi: 1 };
    let note_2 = Note { kind: 0, value: 0, address: 0, psi: 0 };
    let input_note_1 = InputNote { note: note_1, secret_key: pk };
    let input_note_2 = InputNote { note: note_2, secret_key: 0 };

    let note_3 = Note { kind: 1, value: 10, address, psi: 3 };
    let note_4 = Note { kind: 0, value: 0, address: 0, psi: 0 };

    let input_commitment_1 = get_note_commitment(note_1);
    let output_commitment_1 = get_note_commitment(note_3);
    let output_commitment_2 = get_note_commitment(note_4);

    let input_path_1 = MerklePath { path: [0; 160] };
    let input_path_2 = MerklePath { path: [0; 160] };
    let recent_root = input_path_1.root_from_leaf(input_commitment_1) + 1;
    let utxo = Utxo {
        input_notes: [input_note_1, input_note_2],
        output_notes: [note_3, note_4],
        input_merkle_paths: [input_path_1, input_path_2],
    };

    main(
        utxo,
        recent_root,
        [poseidon2::Poseidon2::hash([input_commitment_1, note_1.psi], 2), 0],
        [output_commitment_1, output_commitment_2],
    );
}
