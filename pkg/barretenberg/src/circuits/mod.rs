mod agg_agg;
mod agg_test;
mod agg_utxo;
mod note;
mod points;
mod signature;
#[cfg(test)]
mod tests;
mod utxo;

use std::io::Read;

pub use agg_utxo::*;
use base64::Engine;
use flate2::read::GzDecoder;
pub use utxo::*;

fn get_bytecode_from_program(program_json: &str) -> Vec<u8> {
    let mut program = serde_json::from_str::<serde_json::Value>(program_json).unwrap();
    let bytecode_base64 = program
        .get_mut("bytecode")
        .take()
        .unwrap()
        .as_str()
        .unwrap();
    let bytecode_gzipped = base64::engine::general_purpose::STANDARD
        .decode(bytecode_base64)
        .unwrap();
    let mut decoder = GzDecoder::new(&bytecode_gzipped[..]);
    let mut bytecode = Vec::new();
    decoder.read_to_end(&mut bytecode).unwrap();

    bytecode
}
