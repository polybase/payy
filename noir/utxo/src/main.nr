use common::{
    check_commitment, check_input_note_ownership, get_address, get_note_commitment, InputNote, Note,
};
use poseidon::poseidon2;

mod test;

fn main(
    input_notes: [InputNote; 2],
    output_notes: [Note; 2],
    // TODO: rename to pmessage
    pmessage4: Field,
    commitments: pub [Field; 4],
    messages: pub [Field; 6],
) {
    // Check the commitments
    check_commitment(input_notes[0].note, commitments[0]);
    check_commitment(input_notes[1].note, commitments[1]);
    check_commitment(output_notes[0], commitments[2]);
    check_commitment(output_notes[1], commitments[3]);

    // Check individual outputs are not greater than 240 bits
    output_notes[0].value.assert_max_bit_size::<240>();
    output_notes[1].value.assert_max_bit_size::<240>();

    // TODO: we need to check totals BY KIND

    // We do not need to check the input notes values, as they must already have been added
    // to the tree, and the only way for that to occur if them to be added as an output note first
    // (which the check below would catch). Also, an overflow attack on the inputs would be deterimental
    // to the attacker as they would have to provide more input funds, and would receive less output funds.
    let input_value = input_notes[0].note.value + input_notes[1].note.value;

    // Because we have already checked the individual output notes are no greater than 240 bits,
    // then the maximum of the addition can be 2^241 which means the output value can never overflow
    let output_value = output_notes[0].value + output_notes[1].value;

    // If there are multiple kinds:
    //  - there can be no padding notes
    //  - input[i].kind == output[i].kind (i.e. no re-ordering of kinds allowed)
    let is_input_multiple_kinds = is_multiple_kinds(input_notes[0].note, input_notes[1].note);

    // Kind of UTXO txn (in future we may have subtypes for kind, which define different encodings
    // but for now its easier to define this way, so bytes[32] will always be main kind)
    let kind = messages[0];

    // TODO: add this check back in (in agg_utxo proof)
    // u8 (up to 5 bits, <=32)
    // let kind_bytes = messages[0].to_be_bytes();
    // let kind = kind_bytes[0];

    // Messages (only for mint/burns, otherwise all zeros)
    let msg_note_kind = messages[1];
    let msg_value = messages[2];
    // We only constrain the message hash for mints. For burns, any hash can be used
    // as its only used by the substitutor to early release funds. Early release is
    // secured by the validator, so as long as it records the burn hash it should be valid.
    // A user could provide a burn hash that is already used, which would prevent
    // a the substitutor from substituting the burn; this is acceptable.
    let msg_hash = messages[3];
    let burn_addr = messages[4];

    // Mint hash - we use the private message to prevent others from
    // being able to frontrun the mint hash in their own utxo txn
    let mint_hash = poseidon2::Poseidon2::hash([output_notes[0].psi, output_notes[1].psi], 2);

    // Burn hash - used for the burn substitutor and to verify status of an existing burn -
    // could be any hash so long as its asserted in the proof and cannot be modified. We were
    // planning to use the commitment of both inputs, but that is extra work and not neccessary.
    let burn_hash = commitments[0];

    if (kind == 1) {
        //SEND
        check_totals_match(input_value, output_value);

        // Check the input/output values match
        kinds_match(
            is_input_multiple_kinds,
            input_notes[0].note,
            input_notes[1].note,
            output_notes[0],
            output_notes[1],
        );
    } else if (kind == 2) {
        // MINT
        // Check the second output note is padding or same kind
        check_note_has_kind_or_padding(output_notes[1], output_notes[0].kind);
        // Assert mint value
        assert(output_value == input_value + msg_value, "Mint output must match value message");
        // Assert mint hash
        assert(mint_hash == msg_hash, "Mint hash must match message");
        // Assert note kind
        assert(output_notes[0].kind == msg_note_kind, "Mint note kind must match message")
    } else if (kind == 3) {
        // BURN
        // Prevent frontrunning the txn and changing the evm address
        assert(pmessage4 == burn_addr, "messages[4] must match private input");
        // Assert burn hash
        assert(burn_hash == msg_hash, "Burn hash must match message");
        // Check the second input note is padding or same kind
        assert(!is_input_multiple_kinds, "Inconsistent input kind is not allowed for burn");
        // Assert burn value
        assert(input_value == output_value + msg_value, "Burn output must match value message");
        // Assert burn kind
        assert(input_notes[0].note.kind == msg_note_kind, "Burn note kind must match message")
    } else {
        // TODO: should we create a contract kind?
        assert(false, "Invalid kind");
    }

    // Check the user owns the input notes
    check_input_note_ownership(input_notes[0]);
    check_input_note_ownership(input_notes[1]);
}

fn is_multiple_kinds(note1: Note, note2: Note) -> bool {
    if note2.kind == 0 {
        false
    } else {
        note1.kind != note2.kind
    }
}

fn check_totals_match(input_total: Field, output_total: Field) {
    assert(input_total == output_total, "Input and output totals do not match");
}

/// Check the totals for each kind of note match on the inputs and outputs
fn kinds_match(is_multiple_kinds: bool, input1: Note, input2: Note, output1: Note, output2: Note) {
    if is_multiple_kinds {
        assert(input1.kind == output1.kind, "Inconsistent kind in input/output notes at 0 index");
        assert(input2.kind == output2.kind, "Inconsistent kind in input/output notes at 1 index");
    } else {
        assert(output1.kind == input1.kind, "Inconsistent kind in note");
        check_note_has_kind_or_padding(output2, input1.kind)
    }
}

fn check_note_has_kind_or_padding(note: Note, kind: Field) {
    if note.kind != 0 {
        assert(note.kind == kind, "Inconsistent kind in note");
    }
}

#[test]
fn test_send_two_inputs_two_outputs() {
    let pk: Field = 101;
    let address = get_address(pk);

    // Inputs
    let note_1 = Note { kind: 1, value: 10, address, psi: 1 };
    let note_2 = Note { kind: 1, value: 5, address, psi: 2 };
    let input_note_1 = InputNote { note: note_1, secret_key: pk };
    let input_note_2 = InputNote { note: note_2, secret_key: pk };

    // Outputs
    let note_3 = Note { kind: 1, value: 1, address, psi: 3 };
    let note_4 = Note { kind: 1, value: 14, address, psi: 4 };

    // Commitments
    main(
        [input_note_1, input_note_2],
        [note_3, note_4],
        0,
        [
            get_note_commitment(note_1),
            get_note_commitment(note_2),
            get_note_commitment(note_3),
            get_note_commitment(note_4),
        ],
        [1, 0, 0, 0, 0, 0],
    )
}

#[test]
fn test_send_one_input_two_outputs() {
    let pk: Field = 101;
    let address = get_address(pk);

    // Inputs
    let note_1 = Note { kind: 1, value: 10, address, psi: 1 };
    let note_2 = Note { kind: 0, value: 0, address, psi: 0 };
    let input_note_1 = InputNote { note: note_1, secret_key: pk };
    let input_note_2 = InputNote { note: note_2, secret_key: 0 };

    // Outputs
    let note_3 = Note { kind: 1, value: 1, address, psi: 3 };
    let note_4 = Note { kind: 1, value: 9, address, psi: 4 };

    // Commitments
    main(
        [input_note_1, input_note_2],
        [note_3, note_4],
        0,
        [
            get_note_commitment(note_1),
            get_note_commitment(note_2),
            get_note_commitment(note_3),
            get_note_commitment(note_4),
        ],
        [1, 0, 0, 0, 0, 0],
    )
}

#[test]
fn test_send_two_inputs_one_output() {
    let pk: Field = 101;
    let address = get_address(pk);

    // Inputs
    let note_1 = Note { kind: 1, value: 10, address, psi: 1 };
    let note_2 = Note { kind: 1, value: 5, address, psi: 2 };
    let input_note_1 = InputNote { note: note_1, secret_key: pk };
    let input_note_2 = InputNote { note: note_2, secret_key: pk };

    // Outputs
    let note_3 = Note { kind: 1, value: 15, address, psi: 3 };
    let note_4 = Note { kind: 0, value: 0, address, psi: 0 };

    // Commitments
    main(
        [input_note_1, input_note_2],
        [note_3, note_4],
        0,
        [
            get_note_commitment(note_1),
            get_note_commitment(note_2),
            get_note_commitment(note_3),
            get_note_commitment(note_4),
        ],
        [1, 0, 0, 0, 0, 0],
    )
}

#[test]
fn test_send_diff_kinds() {
    let pk: Field = 101;
    let address = get_address(pk);

    // Inputs
    let note_1 = Note { kind: 1, value: 15, address, psi: 1 };
    let note_2 = Note { kind: 2, value: 5, address, psi: 2 };
    let input_note_1 = InputNote { note: note_1, secret_key: pk };
    let input_note_2 = InputNote { note: note_2, secret_key: pk };

    // Outputs
    let note_3 = Note { kind: 1, value: 15, address, psi: 3 };
    let note_4 = Note { kind: 2, value: 5, address, psi: 4 };

    // Commitments
    main(
        [input_note_1, input_note_2],
        [note_3, note_4],
        0,
        [
            get_note_commitment(note_1),
            get_note_commitment(note_2),
            get_note_commitment(note_3),
            get_note_commitment(note_4),
        ],
        [1, 0, 0, 0, 0, 0],
    )
}

#[test]
fn test_mint() {
    let pk: Field = 101;
    let address = get_address(pk);

    // Inputs
    let note_1 = Note { kind: 1, value: 15, address, psi: 1 };
    let note_2 = Note { kind: 1, value: 5, address, psi: 2 };
    let input_note_1 = InputNote { note: note_1, secret_key: pk };
    let input_note_2 = InputNote { note: note_2, secret_key: pk };

    // Outputs
    let note_3 = Note { kind: 1, value: 0, address: 0, psi: 0 };
    let note_4 = Note { kind: 1, value: 30, address, psi: 3 };

    let commitments = [
        get_note_commitment(note_1),
        get_note_commitment(note_2),
        get_note_commitment(note_3),
        get_note_commitment(note_4),
    ];

    let mint_hash = poseidon2::Poseidon2::hash([note_3.psi, note_4.psi], 2);

    // Commitments
    main(
        [input_note_1, input_note_2],
        [note_3, note_4],
        0,
        commitments,
        [2, 1, 10, mint_hash, 0, 0],
    )
}

#[test]
fn test_burn() {
    let pk: Field = 101;
    let address = get_address(pk);

    // Inputs
    let note_1 = Note { kind: 1, value: 15, address, psi: 1 };
    let note_2 = Note { kind: 1, value: 5, address, psi: 2 };
    let input_note_1 = InputNote { note: note_1, secret_key: pk };
    let input_note_2 = InputNote { note: note_2, secret_key: pk };

    // Outputs
    let note_3 = Note { kind: 0, value: 0, address: 0, psi: 0 };
    let note_4 = Note { kind: 1, value: 10, address, psi: 3 };

    // Commitments
    main(
        [input_note_1, input_note_2],
        [note_3, note_4],
        0,
        [
            get_note_commitment(note_1),
            get_note_commitment(note_2),
            get_note_commitment(note_3),
            get_note_commitment(note_4),
        ],
        [3, 1, 10,  get_note_commitment(note_1), 0, 0],
    )
}

#[test]
fn test_send_zero_value() {
    let pk: Field = 101;
    let address = get_address(pk);

    // Inputs - both zero value notes
    let note_1 = Note { kind: 1, value: 0, address, psi: 1 };
    let note_2 = Note { kind: 0, value: 0, address, psi: 0 };
    let input_note_1 = InputNote { note: note_1, secret_key: pk };
    let input_note_2 = InputNote { note: note_2, secret_key: 0 };

    // Outputs - both zero value notes
    let note_3 = Note { kind: 1, value: 0, address, psi: 3 };
    let note_4 = Note { kind: 0, value: 0, address, psi: 0 };

    main(
        [input_note_1, input_note_2],
        [note_3, note_4],
        0,
        [
            get_note_commitment(note_1),
            get_note_commitment(note_2),
            get_note_commitment(note_3),
            get_note_commitment(note_4),
        ],
        [1, 0, 0, 0, 0, 0],
    )
}

#[test]
fn test_mint_with_padding_input() {
    let pk: Field = 101;
    let address = get_address(pk);

    // Inputs - one real note, one padding
    let note_1 = Note { kind: 1, value: 5, address, psi: 1 };
    let note_2 = Note { kind: 0, value: 0, address, psi: 0 };
    let input_note_1 = InputNote { note: note_1, secret_key: pk };
    let input_note_2 = InputNote { note: note_2, secret_key: 0 };

    // Outputs - note_3 is the first output (minted note), note_4 is padding or same kind
    let note_3 = Note { kind: 1, value: 15, address, psi: 3 };
    let note_4 = Note { kind: 1, value: 0, address: 0, psi: 0 };

    let commitments = [
        get_note_commitment(note_1),
        get_note_commitment(note_2),
        get_note_commitment(note_3),
        get_note_commitment(note_4),
    ];

    let mint_hash = poseidon2::Poseidon2::hash([note_3.psi, note_4.psi], 2);

    main(
        [input_note_1, input_note_2],
        [note_3, note_4],
        0,
        commitments,
        [2, 1, 10, mint_hash, 0, 0],
    )
}

#[test]
fn test_burn_single_input() {
    let pk: Field = 101;
    let address = get_address(pk);

    // Inputs - one real note, one padding
    let note_1 = Note { kind: 1, value: 15, address, psi: 1 };
    let note_2 = Note { kind: 0, value: 0, address, psi: 0 };
    let input_note_1 = InputNote { note: note_1, secret_key: pk };
    let input_note_2 = InputNote { note: note_2, secret_key: 0 };

    // Outputs - burn 10 value, keep 5
    let note_3 = Note { kind: 0, value: 0, address: 0, psi: 0 };
    let note_4 = Note { kind: 1, value: 5, address, psi: 3 };

    main(
        [input_note_1, input_note_2],
        [note_3, note_4],
        0,
        [
            get_note_commitment(note_1),
            get_note_commitment(note_2),
            get_note_commitment(note_3),
            get_note_commitment(note_4),
        ],
        [3, 1, 10, get_note_commitment(note_1), 0, 0],
    )
}

#[test]
fn test_mint_different_kinds() {
    let pk: Field = 101;
    let address = get_address(pk);

    // Inputs - mix of different kinds
    let note_1 = Note { kind: 1, value: 5, address, psi: 1 };
    let note_2 = Note { kind: 2, value: 3, address, psi: 2 };
    let input_note_1 = InputNote { note: note_1, secret_key: pk };
    let input_note_2 = InputNote { note: note_2, secret_key: pk };

    // Outputs - mint kind 3, first note is the minted note
    let note_3 = Note { kind: 3, value: 18, address, psi: 3 };
    let note_4 = Note { kind: 0, value: 0, address: 0, psi: 0 };

    let commitments = [
        get_note_commitment(note_1),
        get_note_commitment(note_2),
        get_note_commitment(note_3),
        get_note_commitment(note_4),
    ];

    let mint_hash = poseidon2::Poseidon2::hash([note_3.psi, note_4.psi], 2);

    main(
        [input_note_1, input_note_2],
        [note_3, note_4],
        0,
        commitments,
        [2, 3, 10, mint_hash, 0, 0],
    )
}

#[test]
fn test_send_large_values() {
    let pk: Field = 101;
    let address = get_address(pk);

    // Test with larger values (but still within 240-bit constraint)
    let large_value = 1000000000;

    // Inputs
    let note_1 = Note { kind: 1, value: large_value, address, psi: 1 };
    let note_2 = Note { kind: 1, value: large_value, address, psi: 2 };
    let input_note_1 = InputNote { note: note_1, secret_key: pk };
    let input_note_2 = InputNote { note: note_2, secret_key: pk };

    // Outputs
    let note_3 = Note { kind: 1, value: large_value + 500000, address, psi: 3 };
    let note_4 = Note { kind: 1, value: large_value - 500000, address, psi: 4 };

    main(
        [input_note_1, input_note_2],
        [note_3, note_4],
        0,
        [
            get_note_commitment(note_1),
            get_note_commitment(note_2),
            get_note_commitment(note_3),
            get_note_commitment(note_4),
        ],
        [1, 0, 0, 0, 0, 0],
    )
}

#[test]
fn test_mint_same_kind() {
    let pk: Field = 101;
    let address = get_address(pk);

    // Inputs - same kind
    let note_1 = Note { kind: 2, value: 5, address, psi: 1 };
    let note_2 = Note { kind: 2, value: 10, address, psi: 2 };
    let input_note_1 = InputNote { note: note_1, secret_key: pk };
    let input_note_2 = InputNote { note: note_2, secret_key: pk };

    // Outputs - mint more of same kind, first note is the minted note
    let note_3 = Note { kind: 2, value: 25, address, psi: 3 };
    let note_4 = Note { kind: 2, value: 0, address: 0, psi: 0 };

    let commitments = [
        get_note_commitment(note_1),
        get_note_commitment(note_2),
        get_note_commitment(note_3),
        get_note_commitment(note_4),
    ];

    let mint_hash = poseidon2::Poseidon2::hash([note_3.psi, note_4.psi], 2);

    main(
        [input_note_1, input_note_2],
        [note_3, note_4],
        0,
        commitments,
        [2, 2, 10, mint_hash, 0, 0],
    )
}

#[test(should_fail)]
fn test_overflow_input_first_note() {
    let pk: Field = 101;
    let address = get_address(pk);

    // Create a value that exceeds 240 bits
    // 2^240 = 1766847064778384329583297500742918515827483896875618958121606201292619776
    let overflow_value = 1766847064778384329583297500742918515827483896875618958121606201292619777;

    // Inputs - first note has overflow value
    let note_1 = Note { kind: 1, value: overflow_value, address, psi: 1 };
    let note_2 = Note { kind: 1, value: 5, address, psi: 2 };
    let input_note_1 = InputNote { note: note_1, secret_key: pk };
    let input_note_2 = InputNote { note: note_2, secret_key: pk };

    // Outputs
    let note_3 = Note { kind: 1, value: 10, address, psi: 3 };
    let note_4 = Note { kind: 1, value: 5, address, psi: 4 };

    // This should fail due to overflow check on first input
    main(
        [input_note_1, input_note_2],
        [note_3, note_4],
        0,
        [
            get_note_commitment(note_1),
            get_note_commitment(note_2),
            get_note_commitment(note_3),
            get_note_commitment(note_4),
        ],
        [1, 0, 0, 0, 0, 0],
    )
}

#[test(should_fail)]
fn test_overflow_input_second_note() {
    let pk: Field = 101;
    let address = get_address(pk);

    // Create a value that exceeds 240 bits
    let overflow_value = 1766847064778384329583297500742918515827483896875618958121606201292619777;

    // Inputs - second note has overflow value
    let note_1 = Note { kind: 1, value: 5, address, psi: 1 };
    let note_2 = Note { kind: 1, value: overflow_value, address, psi: 2 };
    let input_note_1 = InputNote { note: note_1, secret_key: pk };
    let input_note_2 = InputNote { note: note_2, secret_key: pk };

    // Outputs
    let note_3 = Note { kind: 1, value: 10, address, psi: 3 };
    let note_4 = Note { kind: 1, value: 5, address, psi: 4 };

    // This should fail due to overflow check on second input
    main(
        [input_note_1, input_note_2],
        [note_3, note_4],
        0,
        [
            get_note_commitment(note_1),
            get_note_commitment(note_2),
            get_note_commitment(note_3),
            get_note_commitment(note_4),
        ],
        [1, 0, 0, 0, 0, 0],
    )
}

#[test(should_fail)]
fn test_overflow_output_first_note() {
    let pk: Field = 101;
    let address = get_address(pk);

    // Create a value that exceeds 240 bits
    let overflow_value = 1766847064778384329583297500742918515827483896875618958121606201292619777;

    // Inputs
    let note_1 = Note { kind: 1, value: 10, address, psi: 1 };
    let note_2 = Note { kind: 1, value: 5, address, psi: 2 };
    let input_note_1 = InputNote { note: note_1, secret_key: pk };
    let input_note_2 = InputNote { note: note_2, secret_key: pk };

    // Outputs - first output has overflow value
    let note_3 = Note { kind: 1, value: overflow_value, address, psi: 3 };
    let note_4 = Note { kind: 1, value: 5, address, psi: 4 };

    // This should fail due to overflow check on first output
    main(
        [input_note_1, input_note_2],
        [note_3, note_4],
        0,
        [
            get_note_commitment(note_1),
            get_note_commitment(note_2),
            get_note_commitment(note_3),
            get_note_commitment(note_4),
        ],
        [1, 0, 0, 0, 0, 0],
    )
}

#[test(should_fail)]
fn test_overflow_output_second_note() {
    let pk: Field = 101;
    let address = get_address(pk);

    // Create a value that exceeds 240 bits
    let overflow_value = 1766847064778384329583297500742918515827483896875618958121606201292619777;

    // Inputs
    let note_1 = Note { kind: 1, value: 10, address, psi: 1 };
    let note_2 = Note { kind: 1, value: 5, address, psi: 2 };
    let input_note_1 = InputNote { note: note_1, secret_key: pk };
    let input_note_2 = InputNote { note: note_2, secret_key: pk };

    // Outputs - second output has overflow value
    let note_3 = Note { kind: 1, value: 5, address, psi: 3 };
    let note_4 = Note { kind: 1, value: overflow_value, address, psi: 4 };

    // This should fail due to overflow check on second output
    main(
        [input_note_1, input_note_2],
        [note_3, note_4],
        0,
        [
            get_note_commitment(note_1),
            get_note_commitment(note_2),
            get_note_commitment(note_3),
            get_note_commitment(note_4),
        ],
        [1, 0, 0, 0, 0, 0],
    )
}

#[test]
fn test_max_allowed_value() {
    let pk: Field = 101;
    let address = get_address(pk);

    // Use a value that is exactly at the 240-bit limit (2^240 - 1)
    let max_value = 1766847064778384329583297500742918515827483896875618958121606201292619775;

    // Inputs - both notes have max allowed value
    let note_1 = Note { kind: 1, value: max_value, address, psi: 1 };
    let note_2 = Note { kind: 0, value: 0, address, psi: 0 };
    let input_note_1 = InputNote { note: note_1, secret_key: pk };
    let input_note_2 = InputNote { note: note_2, secret_key: 0 };

    // Outputs - transfer to single output
    let note_3 = Note { kind: 1, value: max_value, address, psi: 3 };
    let note_4 = Note { kind: 0, value: 0, address, psi: 0 };

    // This should succeed as the value is exactly at the limit
    main(
        [input_note_1, input_note_2],
        [note_3, note_4],
        0,
        [
            get_note_commitment(note_1),
            get_note_commitment(note_2),
            get_note_commitment(note_3),
            get_note_commitment(note_4),
        ],
        [1, 0, 0, 0, 0, 0],
    )
}
