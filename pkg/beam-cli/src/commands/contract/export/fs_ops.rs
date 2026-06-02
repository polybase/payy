use std::{
    collections::BTreeMap,
    fs::{self, DirEntry},
    path::{Path, PathBuf},
};

use tempfile::Builder;

use crate::commands::contract::error::{Error, Result};

pub(super) fn validate_destination(path: &Path, display: &str) -> Result<()> {
    let Ok(metadata) = fs::symlink_metadata(path) else {
        return Ok(());
    };
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(Error::ExportDestinationInvalid {
            path: display.to_owned(),
        });
    }
    if fs::read_dir(path).map_err(write_failed)?.next().is_some() {
        return Err(Error::ExportDestinationNotEmpty {
            path: display.to_owned(),
        });
    }

    Ok(())
}

pub(super) fn prepare_temp_dir(destination: &Path) -> Result<tempfile::TempDir> {
    let parent = destination.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent).map_err(write_failed)?;
    Builder::new()
        .prefix(".beam-contract-export-")
        .tempdir_in(parent)
        .map_err(write_failed)
}

pub(super) fn write_files(root: &Path, files: &BTreeMap<String, Vec<u8>>) -> Result<()> {
    for (path, bytes) in files {
        let full_path = root.join(path);
        if let Some(parent) = full_path.parent() {
            fs::create_dir_all(parent).map_err(write_failed)?;
        }
        fs::write(full_path, bytes).map_err(write_failed)?;
    }

    Ok(())
}

pub(super) fn commit_into_existing(
    temp_root: &Path,
    destination: &Path,
    display: &str,
) -> Result<()> {
    commit_into_existing_with_hooks(temp_root, destination, display, None, None)
}

#[cfg(test)]
pub(crate) fn commit_into_existing_for_test(
    temp_root: &Path,
    destination: &Path,
    display: &str,
    before_move: Option<&mut dyn FnMut(&Path)>,
    after_move: Option<&mut dyn FnMut(&Path)>,
) -> Result<()> {
    commit_into_existing_with_hooks(temp_root, destination, display, before_move, after_move)
}

pub(super) fn write_failed(err: std::io::Error) -> Error {
    Error::ExportWriteFailed {
        reason: err.to_string(),
    }
}

fn commit_into_existing_with_hooks(
    temp_root: &Path,
    destination: &Path,
    display: &str,
    mut before_move: Option<&mut dyn FnMut(&Path)>,
    mut after_move: Option<&mut dyn FnMut(&Path)>,
) -> Result<()> {
    if fs::read_dir(destination)
        .map_err(write_failed)?
        .next()
        .is_some()
    {
        return Err(Error::ExportDestinationNotEmpty {
            path: display.to_owned(),
        });
    }

    let mut entries = fs::read_dir(temp_root)
        .map_err(write_failed)?
        .collect::<std::io::Result<Vec<_>>>()
        .map_err(write_failed)?;
    entries.sort_by_key(DirEntry::file_name);

    let mut moved = Vec::<PathBuf>::new();
    for entry in entries {
        let target = destination.join(entry.file_name());
        if target_exists(&target).map_err(write_failed)? {
            cleanup_moved(moved);
            return Err(Error::ExportDestinationNotEmpty {
                path: display.to_owned(),
            });
        }
        if let Some(before_move) = before_move.as_deref_mut() {
            before_move(&target);
        }
        if let Err(err) = fs::rename(entry.path(), &target) {
            cleanup_moved(moved);
            return Err(write_failed(err));
        }
        if let Some(after_move) = after_move.as_deref_mut() {
            after_move(&target);
        }
        moved.push(target);
    }

    Ok(())
}

fn target_exists(path: &Path) -> std::io::Result<bool> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(err) => Err(err),
    }
}

fn cleanup_moved(moved: Vec<PathBuf>) {
    for moved_path in moved.into_iter().rev() {
        let _ = remove_path(&moved_path);
    }
}

fn remove_path(path: &Path) -> std::io::Result<()> {
    if path.is_dir() {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    }
}
