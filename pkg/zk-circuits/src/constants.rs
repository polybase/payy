/// Depth of the sparse merkle tree, a smaller tree increases the
/// likihood of collisions
pub const MERKLE_TREE_DEPTH: usize = 161;

pub const UTXO_INPUTS: usize = 2;
pub const UTXO_OUTPUTS: usize = 2;

pub const UTXO_AGG_NUMBER: usize = 3;
pub const UTXO_AGG_LEAVES: usize = UTXO_AGG_NUMBER * (UTXO_INPUTS + UTXO_OUTPUTS);

/// Personalisation to blake to increase entropy
pub const BLAKE_PERSONALISATION: &[u8; 13] = b"Polybase_Seed";

/// Extends PSI entropy
pub const NOTE_RCM_EXT: u8 = 0;
