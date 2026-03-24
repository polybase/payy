use std::io::Write;
use std::path::PathBuf;

pub fn write_to_temp_file(data: &[u8], suffix: &str) -> PathBuf {
    let mut file = tempfile::NamedTempFile::with_suffix(suffix).unwrap();
    file.write_all(data).unwrap();
    file.flush().unwrap();
    let path = file.path().to_path_buf();
    // Deliberately leak the tempfile to prevent deletion
    std::mem::forget(file);
    path
}
