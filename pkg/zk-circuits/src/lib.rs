pub mod circuits;
pub mod execute;
pub mod prove;
pub mod traits;
pub mod util;
pub mod verify;

pub use barretenberg_interface::BbBackend;
pub use barretenberg_interface::error::{Error, Result};
pub use circuits::Proof;
pub use circuits::generated::{AggAggCircuit, AggFinalCircuit, AggUtxoCircuit};
pub use traits::{Circuit, Prove, Verify};

pub use circuits::generated::agg_utxo::VERIFICATION_KEY_HASH as AGG_UTXO_VERIFICATION_KEY_HASH;
