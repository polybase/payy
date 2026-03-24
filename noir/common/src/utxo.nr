use poseidon::poseidon2;
use crate::note::{check_input_note_ownership, get_note_commitment, is_multiple_kinds};
use crate::{InputNote, MerklePath, Note};

pub struct Utxo {
    pub input_notes: [InputNote; 2],
    pub output_notes: [Note; 2],
    pub input_merkle_paths: [MerklePath; 2],
}

impl Utxo {
    pub fn validate(
        self,
        recent_root: Field,
        input_nullifiers: [Field; 2],
        output_commitments: [Field; 2],
    ) {
        let input_commitments = [
            get_note_commitment(self.input_notes[0].note),
            get_note_commitment(self.input_notes[1].note),
        ];
        let computed_output_commitments = [
            get_note_commitment(self.output_notes[0]),
            get_note_commitment(self.output_notes[1]),
        ];

        assert(output_commitments[0] == computed_output_commitments[0], "Invalid output commitment 0");
        assert(output_commitments[1] == computed_output_commitments[1], "Invalid output commitment 1");

        assert(
            input_nullifiers[0] == expected_nullifier(self.input_notes[0].note, input_commitments[0]),
            "Invalid nullifier 0",
        );
        assert(
            input_nullifiers[1] == expected_nullifier(self.input_notes[1].note, input_commitments[1]),
            "Invalid nullifier 1",
        );

        self.output_notes[0].value.assert_max_bit_size::<240>();
        self.output_notes[1].value.assert_max_bit_size::<240>();

        verify_input_membership(input_commitments[0], self.input_merkle_paths[0], recent_root);
        verify_input_membership(input_commitments[1], self.input_merkle_paths[1], recent_root);

        check_input_note_ownership(self.input_notes[0]);
        check_input_note_ownership(self.input_notes[1]);

        let (is_multiple_kinds, _) = is_multiple_kinds([
            self.input_notes[0].note,
            self.input_notes[1].note,
            self.output_notes[0],
            self.output_notes[1],
        ]);
        assert(!is_multiple_kinds, "Inconsistent kinds are not allowed");
    }
}

fn expected_nullifier(note: Note, commitment: Field) -> Field {
    if commitment == 0 { 0 } else { poseidon2::Poseidon2::hash([commitment, note.psi], 2) }
}

fn verify_input_membership(commitment: Field, merkle_path: MerklePath, recent_root: Field) {
    if commitment != 0 {
        assert(merkle_path.validate(commitment, recent_root), "Input commitment is not in recent root");
    }
}
