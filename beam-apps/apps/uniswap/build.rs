use std::env;
use std::fs;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-env-changed=BEAM_UNISWAP_PUBLIC_API_KEY");

    let key = match env::var("BEAM_UNISWAP_PUBLIC_API_KEY") {
        Ok(value) => value,
        Err(env::VarError::NotPresent) => String::new(),
        Err(err) => return Err(Box::new(err)),
    };
    let out_dir = PathBuf::from(env::var("OUT_DIR")?);
    let generated = format!("pub const BEAM_UNISWAP_PUBLIC_API_KEY: &str = {:?};\n", key);

    fs::write(out_dir.join("public_api_key.rs"), generated)?;
    Ok(())
}
