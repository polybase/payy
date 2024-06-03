mod circuit;
mod input_note;
mod note;
pub mod proof;
#[cfg(test)]
mod tests;
#[allow(clippy::module_inception)]
mod utxo;

/// UTXO circuit runs on the client, it takes a set of input notes, validates a given merkle path
/// (which will later be used to prove they are in the tree), and outputs a set of output notes
///
///  (Private)                            (Public)
///                    ┌────────┐         
///  InputNote         │        │         Nullifer (for InputNote)
///  InputNote  ───►   │  UTXO  │  ───►   Nullifer (for InputNote)
///  Note (output)     │        │         Note (output) commitment
///  Note (output)     │        │         Note (output) commitment
///                    │        │         Recent Merkle Root (used to verify Input Notes)
///                    └────────┘
///
// Main circuit
pub use utxo::*;

// Input/output notes to utxo txn
pub use input_note::*;
pub use note::*;
