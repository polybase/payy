use common::{check_commitment, check_input_note_ownership, get_address, InputNote, Note};
use poseidon::poseidon2;

fn main(
    notes: [Note; 10],
    secret_keys: [Field; 10],
    address: Field,
    timestamp: pub Field,
    value: pub Field,
    hash: pub Field,
    commitments: pub [Field; 10],
) {
    let mut notes_value = 0;

    for i in 0..10 {
        let note = notes[i];
        let secret_key = secret_keys[i];
        let commitment = commitments[i];

        check_commitment(note, commitment);
        check_input_note_ownership(InputNote { note, secret_key });

        notes_value += note.value;
    }

    assert(value == notes_value, "Invalid value");
    assert(poseidon2::Poseidon2::hash([timestamp, address], 2) == hash);
}

#[test]
fn test_main() {
    let pk = 101;
    let address = get_address(pk);

    // Create 10 notes with the same address
    let mut notes: [Note; 10] = [
        Note { kind: 0, value: 0, address: 0, psi: 0 },
        Note { kind: 1, value: 20, address: address, psi: 2 },
        Note { kind: 1, value: 30, address: address, psi: 3 },
        Note { kind: 1, value: 40, address: address, psi: 4 },
        Note { kind: 1, value: 50, address: address, psi: 5 },
        Note { kind: 1, value: 60, address: address, psi: 6 },
        Note { kind: 1, value: 70, address: address, psi: 7 },
        Note { kind: 1, value: 80, address: address, psi: 8 },
        Note { kind: 1, value: 90, address: address, psi: 9 },
        Note { kind: 1, value: 100, address: address, psi: 10 },
    ];

    // Calculate the total value of all notes
    let total_value = 540;

    // Create an array of secret keys (all the same in this test)
    let mut secret_keys = [pk; 10];
    secret_keys[0] = 0;

    // Calculate commitments for each note
    let mut commitments: [Field; 10] = [0; 10];
    for i in 0..10 {
        commitments[i] = common::get_note_commitment(notes[i]);
    }

    // Create timestamp and calculate hash
    let timestamp = 10000000;
    let message_hash = poseidon2::Poseidon2::hash([timestamp, address], 2);

    main(
        notes,
        secret_keys,
        address,
        timestamp,
        total_value,
        message_hash,
        commitments,
    )
}
