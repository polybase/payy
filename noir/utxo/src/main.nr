use common::{
    check_commitment, check_input_note_ownership, get_address, get_note_commitment, is_multiple_kinds, InputNote,
    Note,
};
use poseidon::poseidon2;

mod test;

fn main(
    input_notes: [InputNote; 2],
    output_notes: [Note; 2],
    pmessage4: Field,
    commitments: pub [Field; 4],
    messages: pub [Field; 5],
) {
    // Check the commitments
    check_commitment(input_notes[0].note, commitments[0]);
    check_commitment(input_notes[1].note, commitments[1]);
    check_commitment(output_notes[0], commitments[2]);
    check_commitment(output_notes[1], commitments[3]);

    // Check individual outputs are not greater than 240 bits
    output_notes[0].value.assert_max_bit_size::<240>();
    output_notes[1].value.assert_max_bit_size::<240>();

    // We do not need to check the input notes values, as they must already have been added
    // to the tree, and the only way for that to occur if them to be added as an output note first
    // (which the check below would catch). Also, an overflow attack on the inputs would be deterimental
    // to the attacker as they would have to provide more input funds, and would receive less output funds.
    let input_value = input_notes[0].note.value + input_notes[1].note.value;

    // Because we have already checked the individual output notes are no greater than 240 bits,
    // then the maximum of the addition can be 2^241 which means the output value can never overflow
    let output_value = output_notes[0].value + output_notes[1].value;

    // Check for multiple kinds, not currently supported in this proof
    let (is_multiple_kinds, _note_kind) = is_multiple_kinds([
        input_notes[0].note,
        input_notes[1].note,
        output_notes[0],
        output_notes[1],
    ]);
    assert(!is_multiple_kinds, "Inconsistent kinds are not allowed");

    // Kind of UTXO txn (in future we may have subtypes for kind, which define different encodings
    // but for now its easier to define this way, so bytes[32] will always be main kind)
    let kind = messages[0];

    // Messages (only for mint/burns, otherwise all zeros)
    let msg_note_kind = messages[1];
    let msg_value = messages[2];

    // Assert message value size is max 240 bits to prevent overflow attack
    msg_value.assert_max_bit_size::<240>();

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
        assert(input_value == output_value, "Input and output totals do not match");
    } else if (kind == 2) {
        // MINT
        // Assert mint utxo is balanced:
        //   - `output_value` is checked above
        //   - `input_value` is checked as it must have previously been an output value
        //   - `msg_value` is checked above (but also using that to overflow would be detrimental to the
        //      attacker)
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
        // Assert burn utxo is balanced:
        //   - `output_value` is checked above
        //   - `input_value` is checked as it must have previously been an output value
        //   - `msg_value` is checked above
        assert(input_value == output_value + msg_value, "Burn output must match value message");
        // Assert burn kind
        assert(input_notes[0].note.kind == msg_note_kind, "Burn note kind must match message")
    } else {
        assert(false, "Invalid kind");
    }

    // Check the user owns the input notes
    check_input_note_ownership(input_notes[0]);
    check_input_note_ownership(input_notes[1]);
}

fn bridged_note_kind() -> Field {
    3533694129556768672311144317398675444585744224105014452550528428861358080
}

#[test]
fn test_send_two_inputs_two_outputs() {
    let pk: Field = 101;
    let address = get_address(pk);

    // Inputs
    let note_1 = Note { kind: bridged_note_kind(), value: 10, address, psi: 1 };
    let note_2 = Note { kind: bridged_note_kind(), value: 5, address, psi: 2 };
    let input_note_1 = InputNote { note: note_1, secret_key: pk };
    let input_note_2 = InputNote { note: note_2, secret_key: pk };

    // Outputs
    let note_3 = Note { kind: bridged_note_kind(), value: 1, address, psi: 3 };
    let note_4 = Note { kind: bridged_note_kind(), value: 14, address, psi: 4 };

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
        [1, 0, 0, 0, 0],
    )
}

#[test]
fn test_send_one_input_two_outputs() {
    let pk: Field = 101;
    let address = get_address(pk);

    // Inputs
    let note_1 = Note { kind: bridged_note_kind(), value: 10, address, psi: 1 };
    let note_2 = Note { kind: 0, value: 0, address, psi: 0 };
    let input_note_1 = InputNote { note: note_1, secret_key: pk };
    let input_note_2 = InputNote { note: note_2, secret_key: 0 };

    // Outputs
    let note_3 = Note { kind: bridged_note_kind(), value: 1, address, psi: 3 };
    let note_4 = Note { kind: bridged_note_kind(), value: 9, address, psi: 4 };

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
        [1, 0, 0, 0, 0],
    )
}

#[test]
fn test_send_two_inputs_one_output() {
    let pk: Field = 101;
    let address = get_address(pk);

    // Inputs
    let note_1 = Note { kind: bridged_note_kind(), value: 10, address, psi: 1 };
    let note_2 = Note { kind: bridged_note_kind(), value: 5, address, psi: 2 };
    let input_note_1 = InputNote { note: note_1, secret_key: pk };
    let input_note_2 = InputNote { note: note_2, secret_key: pk };

    // Outputs
    let note_3 = Note { kind: bridged_note_kind(), value: 15, address, psi: 3 };
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
        [1, 0, 0, 0, 0],
    )
}

#[test(should_fail)]
fn test_send_diff_kinds() {
    let pk: Field = 101;
    let address = get_address(pk);

    // Inputs
    let note_1 = Note { kind: bridged_note_kind(), value: 15, address, psi: 1 };
    let note_2 = Note { kind: 2, value: 5, address, psi: 2 };
    let input_note_1 = InputNote { note: note_1, secret_key: pk };
    let input_note_2 = InputNote { note: note_2, secret_key: pk };

    // Outputs
    let note_3 = Note { kind: bridged_note_kind(), value: 15, address, psi: 3 };
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
        [1, 0, 0, 0, 0],
    )
}

#[test]
fn test_mint() {
    let pk: Field = 101;
    let address = get_address(pk);

    // Inputs
    let note_1 = Note { kind: bridged_note_kind(), value: 15, address, psi: 1 };
    let note_2 = Note { kind: bridged_note_kind(), value: 5, address, psi: 2 };
    let input_note_1 = InputNote { note: note_1, secret_key: pk };
    let input_note_2 = InputNote { note: note_2, secret_key: pk };

    // Outputs
    let note_3 = Note { kind: bridged_note_kind(), value: 0, address: 0, psi: 0 };
    let note_4 = Note { kind: bridged_note_kind(), value: 30, address, psi: 3 };

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
        [2, bridged_note_kind(), 10, mint_hash, 0],
    )
}

#[test]
fn test_burn() {
    let pk: Field = 101;
    let address = get_address(pk);

    // Inputs
    let note_1 = Note { kind: bridged_note_kind(), value: 15, address, psi: 1 };
    let note_2 = Note { kind: bridged_note_kind(), value: 5, address, psi: 2 };
    let input_note_1 = InputNote { note: note_1, secret_key: pk };
    let input_note_2 = InputNote { note: note_2, secret_key: pk };

    // Outputs
    let note_3 = Note { kind: 0, value: 0, address: 0, psi: 0 };
    let note_4 = Note { kind: bridged_note_kind(), value: 10, address, psi: 3 };

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
        [3, bridged_note_kind(), 10, get_note_commitment(note_1), 0],
    )
}

#[test]
fn test_send_zero_value() {
    let pk: Field = 101;
    let address = get_address(pk);

    // Inputs - both zero value notes
    let note_1 = Note { kind: bridged_note_kind(), value: 0, address, psi: 1 };
    let note_2 = Note { kind: 0, value: 0, address, psi: 0 };
    let input_note_1 = InputNote { note: note_1, secret_key: pk };
    let input_note_2 = InputNote { note: note_2, secret_key: 0 };

    // Outputs - both zero value notes
    let note_3 = Note { kind: bridged_note_kind(), value: 0, address, psi: 3 };
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
        [1, 0, 0, 0, 0],
    )
}

#[test]
fn test_mint_with_padding_input() {
    let pk: Field = 101;
    let address = get_address(pk);

    // Inputs - one real note, one padding
    let note_1 = Note { kind: bridged_note_kind(), value: 5, address, psi: 1 };
    let note_2 = Note { kind: 0, value: 0, address, psi: 0 };
    let input_note_1 = InputNote { note: note_1, secret_key: pk };
    let input_note_2 = InputNote { note: note_2, secret_key: 0 };

    // Outputs - note_3 is the first output (minted note), note_4 is padding or same kind
    let note_3 = Note { kind: bridged_note_kind(), value: 15, address, psi: 3 };
    let note_4 = Note { kind: bridged_note_kind(), value: 0, address: 0, psi: 0 };

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
        [2, bridged_note_kind(), 10, mint_hash, 0],
    )
}

#[test]
fn test_burn_single_input() {
    let pk: Field = 101;
    let address = get_address(pk);

    // Inputs - one real note, one padding
    let note_1 = Note { kind: bridged_note_kind(), value: 15, address, psi: 1 };
    let note_2 = Note { kind: 0, value: 0, address, psi: 0 };
    let input_note_1 = InputNote { note: note_1, secret_key: pk };
    let input_note_2 = InputNote { note: note_2, secret_key: 0 };

    // Outputs - burn 10 value, keep 5
    let note_3 = Note { kind: 0, value: 0, address: 0, psi: 0 };
    let note_4 = Note { kind: bridged_note_kind(), value: 5, address, psi: 3 };

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
        [3, bridged_note_kind(), 10, get_note_commitment(note_1), 0],
    )
}
#[test(should_fail)]
fn test_mint_different_kinds() {
    let pk: Field = 101;
    let address = get_address(pk);

    // Inputs - mix of different kinds
    let note_1 = Note { kind: bridged_note_kind(), value: 5, address, psi: 1 };
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
        [2, 3, 10, mint_hash, 0],
    )
}

#[test]
fn test_send_large_values() {
    let pk: Field = 101;
    let address = get_address(pk);

    // Test with larger values (but still within 240-bit constraint)
    let large_value = 1000000000;

    // Inputs
    let note_1 = Note { kind: bridged_note_kind(), value: large_value, address, psi: 1 };
    let note_2 = Note { kind: bridged_note_kind(), value: large_value, address, psi: 2 };
    let input_note_1 = InputNote { note: note_1, secret_key: pk };
    let input_note_2 = InputNote { note: note_2, secret_key: pk };

    // Outputs
    let note_3 = Note { kind: bridged_note_kind(), value: large_value + 500000, address, psi: 3 };
    let note_4 = Note { kind: bridged_note_kind(), value: large_value - 500000, address, psi: 4 };

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
        [1, 0, 0, 0, 0],
    )
}

#[test]
fn test_mint_same_kind() {
    let pk: Field = 101;
    let address = get_address(pk);

    // Inputs - same kind
    let note_1 = Note { kind: bridged_note_kind(), value: 5, address, psi: 1 };
    let note_2 = Note { kind: bridged_note_kind(), value: 10, address, psi: 2 };
    let input_note_1 = InputNote { note: note_1, secret_key: pk };
    let input_note_2 = InputNote { note: note_2, secret_key: pk };

    // Outputs - mint more of same kind, first note is the minted note
    let note_3 = Note { kind: bridged_note_kind(), value: 25, address, psi: 3 };
    let note_4 = Note { kind: bridged_note_kind(), value: 0, address: 0, psi: 0 };

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
        [2, bridged_note_kind(), 10, mint_hash, 0],
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
    let note_1 = Note { kind: bridged_note_kind(), value: overflow_value, address, psi: 1 };
    let note_2 = Note { kind: bridged_note_kind(), value: 5, address, psi: 2 };
    let input_note_1 = InputNote { note: note_1, secret_key: pk };
    let input_note_2 = InputNote { note: note_2, secret_key: pk };

    // Outputs
    let note_3 = Note { kind: bridged_note_kind(), value: 10, address, psi: 3 };
    let note_4 = Note { kind: bridged_note_kind(), value: 5, address, psi: 4 };

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
        [1, 0, 0, 0, 0],
    )
}

#[test(should_fail)]
fn test_overflow_input_second_note() {
    let pk: Field = 101;
    let address = get_address(pk);

    // Create a value that exceeds 240 bits
    let overflow_value = 1766847064778384329583297500742918515827483896875618958121606201292619777;

    // Inputs - second note has overflow value
    let note_1 = Note { kind: bridged_note_kind(), value: 5, address, psi: 1 };
    let note_2 = Note { kind: bridged_note_kind(), value: overflow_value, address, psi: 2 };
    let input_note_1 = InputNote { note: note_1, secret_key: pk };
    let input_note_2 = InputNote { note: note_2, secret_key: pk };

    // Outputs
    let note_3 = Note { kind: bridged_note_kind(), value: 10, address, psi: 3 };
    let note_4 = Note { kind: bridged_note_kind(), value: 5, address, psi: 4 };

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
        [1, 0, 0, 0, 0],
    )
}

#[test(should_fail)]
fn test_overflow_output_first_note() {
    let pk: Field = 101;
    let address = get_address(pk);

    // Create a value that exceeds 240 bits
    let overflow_value = 1766847064778384329583297500742918515827483896875618958121606201292619777;

    // Inputs
    let note_1 = Note { kind: bridged_note_kind(), value: 10, address, psi: 1 };
    let note_2 = Note { kind: bridged_note_kind(), value: 5, address, psi: 2 };
    let input_note_1 = InputNote { note: note_1, secret_key: pk };
    let input_note_2 = InputNote { note: note_2, secret_key: pk };

    // Outputs - first output has overflow value
    let note_3 = Note { kind: bridged_note_kind(), value: overflow_value, address, psi: 3 };
    let note_4 = Note { kind: bridged_note_kind(), value: 5, address, psi: 4 };

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
        [1, 0, 0, 0, 0],
    )
}

#[test(should_fail)]
fn test_overflow_output_second_note() {
    let pk: Field = 101;
    let address = get_address(pk);

    // Create a value that exceeds 240 bits
    let overflow_value = 1766847064778384329583297500742918515827483896875618958121606201292619777;

    // Inputs
    let note_1 = Note { kind: bridged_note_kind(), value: 10, address, psi: 1 };
    let note_2 = Note { kind: bridged_note_kind(), value: 5, address, psi: 2 };
    let input_note_1 = InputNote { note: note_1, secret_key: pk };
    let input_note_2 = InputNote { note: note_2, secret_key: pk };

    // Outputs - second output has overflow value
    let note_3 = Note { kind: bridged_note_kind(), value: 5, address, psi: 3 };
    let note_4 = Note { kind: bridged_note_kind(), value: overflow_value, address, psi: 4 };

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
        [1, 0, 0, 0, 0],
    )
}

#[test]
fn test_max_allowed_value() {
    let pk: Field = 101;
    let address = get_address(pk);

    // Use a value that is exactly at the 240-bit limit (2^240 - 1)
    let max_value = 1766847064778384329583297500742918515827483896875618958121606201292619775;

    // Inputs - both notes have max allowed value
    let note_1 = Note { kind: bridged_note_kind(), value: max_value, address, psi: 1 };
    let note_2 = Note { kind: 0, value: 0, address, psi: 0 };
    let input_note_1 = InputNote { note: note_1, secret_key: pk };
    let input_note_2 = InputNote { note: note_2, secret_key: 0 };

    // Outputs - transfer to single output
    let note_3 = Note { kind: bridged_note_kind(), value: max_value, address, psi: 3 };
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
        [1, 0, 0, 0, 0],
    )
}

#[test(should_fail)]
fn test_msg_value_overflow() {
    let pk: Field = 101;
    let address = get_address(pk);

    // Create a value that exceeds 240 bits
    let overflow_value = 1766847064778384329583297500742918515827483896875618958121606201292619777;

    // Inputs
    let note_1 = Note { kind: bridged_note_kind(), value: 10, address, psi: 1 };
    let note_2 = Note { kind: bridged_note_kind(), value: 5, address, psi: 2 };
    let input_note_1 = InputNote { note: note_1, secret_key: pk };
    let input_note_2 = InputNote { note: note_2, secret_key: pk };

    // Outputs - mint with overflow msg_value
    let note_3 = Note { kind: bridged_note_kind(), value: 15, address, psi: 3 };
    let note_4 = Note { kind: 0, value: 0, address: 0, psi: 0 };

    let commitments = [
        get_note_commitment(note_1),
        get_note_commitment(note_2),
        get_note_commitment(note_3),
        get_note_commitment(note_4),
    ];

    let mint_hash = poseidon2::Poseidon2::hash([note_3.psi, note_4.psi], 2);

    // This should fail due to msg_value overflow check
    main(
        [input_note_1, input_note_2],
        [note_3, note_4],
        0,
        commitments,
        [2, bridged_note_kind(), overflow_value, mint_hash, 0],
    )
}
