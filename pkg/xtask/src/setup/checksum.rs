use std::fs;
use std::io::Read;
use std::path::Path;

use contextful::ResultContextExt;
use sha2::{Digest, Sha256};

use crate::error::{Result, XTaskError};

pub fn verify_sha256(path: &Path, expected: &'static str) -> Result<()> {
    let mut file =
        fs::File::open(path).with_context(|| format!("open {} for checksum", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 32 * 1024];

    loop {
        let read = file
            .read(&mut buf)
            .with_context(|| format!("read {} for checksum", path.display()))?;
        if read == 0 {
            break;
        }
        hasher.update(&buf[..read]);
    }

    let digest = hasher.finalize();
    let digest_hex = hex::encode(digest);

    if digest_hex != expected {
        return Err(XTaskError::ChecksumMismatch {
            path: path.to_path_buf(),
            expected,
            actual: digest_hex,
        });
    }

    Ok(())
}
