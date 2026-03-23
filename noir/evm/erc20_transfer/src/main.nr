mod eip1559transaction;
mod signature;

use eip1559transaction::Type2Transaction;
use common::{Address, MerklePath};
use poseidon::poseidon2::Poseidon2;
use signature::Signature;

/// We use Poseidon hashes that operate on the field we use for proving
type Hash = Field;

pub struct Note {
    pub kind: Field,
    pub address: Address,
    pub token: Address,
    pub value: Field,
    pub psi: Field,
}

impl Note {
    pub fn hash(self) -> Field {
        Poseidon2::hash(
            [
                self.kind,
                Field::from_be_bytes(self.address),
                Field::from_be_bytes(self.token),
                self.value,
                self.psi,
            ],
            5,
        )
    }
}

/// Shows that somewhere in the Merkle tree represented by `input_note_merkle_root`,
/// there are notes belonging to an account that has signed an ERC-20 transfer.
/// Specifically, the notes are sufficient to pay for that transfer, output_notes[1]
/// is a note for the recipient and output_notes[0] holds any leftover change.
///
/// The consumer of the proof should
///  - add `new_commitments` to the Merkle tree so it is
///    possible to use the output notes in the future.
///  - ensure that `new_nullifiers` haven't been seen before and
///    aren't identical! They are the only mechanism preventing double spend.
///  - check that chain_id and bridge_address to prevent cross-chain replay attacks.
fn main(
    // Needed to prevent cross-chain or cross-contract attacks
    chain_id: pub u64,
    bridge_address: pub Field,
    // Ethereum transaction
    original_transfer: ERC20Transfer,
    signature: Signature,
    // Input notes
    input_notes: [Note; 2],
    input_note_merkle_proofs: [MerklePath; 2],
    input_note_merkle_root: pub Hash,
    new_nullifiers: pub [Hash; 3],
    // Output notes
    output_notes: [Note; 2],
    new_commitments: pub [Hash; 2],
) {
    let transaction_hash = original_transfer.hash();
    assert(signature.matches_transaction_hash(transaction_hash), "Invalid signature");
    let sender = signature.sender_address();

    // The same transaction must not be used twice to prevent replay attacks or accidents
    // This prevents cross-chain use of the same signed transaction
    assert(chain_id == original_transfer.chain_id);
    // We cannot prevent cross-contract use of the same transaction, as it cannot
    // indicate what bridge to use but at least we can make the proof valid only for one.
    // The signature's r is a random number, so it is perfect for building the nullifier.
    // s would be a poor choice because if (r, s) is a valid signature, (r, -s) is, too.
    assert(
        new_nullifiers[2]
            == Poseidon2::hash(
                [Field::from_be_bytes(signature.r), Field::from(chain_id), bridge_address],
                3,
            ),
    );

    for i in 0..input_notes.len() {
        let note = input_notes[i];
        assert(note.address == sender);
        assert(note.token == original_transfer.token_address);

        let commitment = note.hash();
        assert(input_note_merkle_proofs[i].validate(commitment, input_note_merkle_root));

        // TODO(ENG-6670) is hashing with psi a good way to build the nullifier?
        let nullifier = Poseidon2::hash([commitment, note.psi], 2);
        assert(new_nullifiers[i] == nullifier)
        // The PrivacyBridge is responsible for checking that the nullifiers do not already exist
    }

    for i in 0..output_notes.len() {
        let note = output_notes[i];
        assert(note.token == original_transfer.token_address);

        let commitment = note.hash();
        assert(new_commitments[i] == commitment);
    }

    // This addition can overflow but that means the sender loses money
    let value_in = input_notes[0].value + input_notes[1].value;

    // The first output note is for the sender's leftover tokens
    assert(output_notes[0].address == sender);
    assert(!value_in.lt(original_transfer.amount), "Not enough funds for transfer");
    assert(output_notes[0].value == value_in - original_transfer.amount);

    // The second one is for the receiver
    assert(output_notes[1].address == original_transfer.receiver);
    assert(output_notes[1].value == original_transfer.amount);
}

/// Parsed EIP-1559 transaction that calls ERC-20.transfer
pub struct ERC20Transfer {
    chain_id: u64,
    nonce: u64,
    max_priority_fee_per_gas: u128,
    max_fee_per_gas: u128,
    gas_limit: u64,

    token_address: Address,
    receiver: Address,
    amount: Field,
}

impl ERC20Transfer {
    fn hash(self) -> [u8; 32] {
        let Self {
            chain_id,
            nonce,
            max_priority_fee_per_gas,
            max_fee_per_gas,
            gas_limit,
            token_address,
            receiver,
            amount,
        } = self;

        // selector for transfer(address,uint256)
        let transfer_selector = [0xa9, 0x05, 0x9c, 0xbb];
        let receiver = [0; 12].concat(receiver);
        let amount = amount.to_be_bytes::<32>();

        Type2Transaction {
            chain_id,
            nonce,
            max_priority_fee_per_gas,
            max_fee_per_gas,
            gas_limit,
            value: 0,
            to: token_address,
            data: transfer_selector.concat(receiver).concat(amount),
        }
            .hash()
    }
}
