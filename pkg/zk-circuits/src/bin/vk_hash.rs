use acvm::AcirField;
use bn254_blackbox_solver::poseidon_hash;
use element::{Base, Element};
use std::env;
use std::fs;
use std::path::Path;
use zk_circuits::verify::VerificationKey;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("Usage: {} <vk_fields.json>", args[0]);
        std::process::exit(1);
    }

    let vk_fields_path = &args[1];

    if !Path::new(vk_fields_path).exists() {
        eprintln!("Error: File {vk_fields_path} does not exist");
        std::process::exit(1);
    }

    let vk_fields_data = match fs::read(vk_fields_path) {
        Ok(data) => data,
        Err(e) => {
            eprintln!("Error reading file {vk_fields_path}: {e}");
            std::process::exit(1);
        }
    };

    let vk_fields: Vec<String> = match serde_json::from_slice(&vk_fields_data) {
        Ok(fields) => fields,
        Err(e) => {
            eprintln!("Error parsing JSON from {vk_fields_path}: {e}");
            std::process::exit(1);
        }
    };

    let vk_fields = vk_fields
        .into_iter()
        .map(|field| {
            Base::from_hex(&field).unwrap_or_else(|| {
                eprintln!("Error parsing hex field {field}");
                std::process::exit(1);
            })
        })
        .collect::<Vec<_>>();

    let verification_key = VerificationKey(vk_fields);

    let hash = match poseidon_hash(&verification_key.0) {
        Ok(hash) => hash,
        Err(e) => {
            eprintln!("Error computing Poseidon hash: {e}");
            std::process::exit(1);
        }
    };

    let hash_u256 = Element::from_base(hash).to_u256();
    let hash_hex = format!("0x{hash_u256:064x}");

    println!("u256: {hash_u256}");
    println!("hex: {hash_hex}");
}
