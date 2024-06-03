use std::env::var;
use std::fs;
use std::path::{Path, PathBuf};

use wire_message::WireMessage;

use crate::data::SnarkWitness;

pub fn save_witness(name: &str, snark_witness: &SnarkWitness) {
    save_file(name, snark_witness);
}

pub fn load_witness(name: &str) -> Option<SnarkWitness> {
    load_file(name)
}

pub fn save_file(name: &str, data: &impl WireMessage) {
    let dir = get_dir();
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join(format!("{name}.proof"));
    fs::write(path, data.to_bytes().unwrap()).unwrap();
}

pub fn load_file<M: WireMessage>(name: &str) -> Option<M> {
    let dir = get_dir();
    let path = dir.join(format!("{name}.proof"));
    println!("Loading proof from: {path:?}");
    let bytes = fs::read(path).ok()?;
    M::from_bytes(&bytes).ok()
}

pub fn get_dir() -> PathBuf {
    var("PROOF_DIR")
        .map(PathBuf::from)
        .ok()
        .or(find_workspace_root())
        .map(|dir| dir.join("fixtures/proofs"))
        .unwrap_or_else(|| PathBuf::from("./proofs"))
}

fn find_workspace_root() -> Option<PathBuf> {
    if let Ok(package_dir) = var("CARGO_MANIFEST_DIR") {
        let mut path = Path::new(&package_dir).to_path_buf();

        while let Some(parent_path) = path.parent() {
            let possible_root = parent_path.join("Cargo.toml");

            if possible_root.exists() && fs::metadata(&possible_root).unwrap().is_file() {
                return Some(parent_path.to_path_buf());
            } else {
                path = parent_path.to_path_buf()
            }
        }
    }

    None
}
