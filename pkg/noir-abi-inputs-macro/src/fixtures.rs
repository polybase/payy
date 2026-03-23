use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use proc_macro2::Span;
use syn::LitStr;

use crate::{AbiField, AbiType, ProgramArtifact, StructPath};

pub(crate) struct FixturePaths {
    pub(crate) program_path: PathBuf,
    pub(crate) key_path: PathBuf,
    pub(crate) key_fields_path: PathBuf,
}

/// Resolve a path from a literal string, relative to CARGO_MANIFEST_DIR if not absolute.
pub(crate) fn resolve_path(path_literal: &LitStr) -> syn::Result<PathBuf> {
    let raw = path_literal.value();
    let path = Path::new(&raw);
    if path.is_absolute() {
        return Ok(path.to_path_buf());
    }

    let manifest_dir = env::var("CARGO_MANIFEST_DIR").map_err(|err| {
        syn::Error::new(
            path_literal.span(),
            format!("CARGO_MANIFEST_DIR is not set: {}", err),
        )
    })?;

    Ok(Path::new(&manifest_dir).join(path))
}

pub(crate) fn resolve_fixture_paths(path_literal: &LitStr) -> syn::Result<FixturePaths> {
    let fixture_dir = resolve_path(path_literal)?;

    Ok(FixturePaths {
        program_path: fixture_dir.join("program.json"),
        key_path: fixture_dir.join("key"),
        key_fields_path: fixture_dir.join("key_fields.json"),
    })
}

pub(crate) fn parse_program(path: &Path) -> syn::Result<ProgramArtifact> {
    let contents = fs::read_to_string(path).map_err(|err| {
        syn::Error::new(
            Span::call_site(),
            format!("failed to read program json {}: {}", path.display(), err),
        )
    })?;
    serde_json::from_str(&contents).map_err(|err| {
        syn::Error::new(
            Span::call_site(),
            format!("failed to parse program json {}: {}", path.display(), err),
        )
    })
}

pub(crate) fn collect_structs(ty: &AbiType, out: &mut BTreeMap<StructPath, Vec<AbiField>>) {
    match ty {
        AbiType::Struct { path, fields } => {
            if !out.contains_key(path) {
                out.insert(path.clone(), fields.clone());
                for field in fields {
                    collect_structs(&field.ty, out);
                }
            }
        }
        AbiType::Array { ty, .. } => collect_structs(ty, out),
        AbiType::Tuple { fields } => {
            for field in fields {
                collect_structs(field, out);
            }
        }
        AbiType::Field => {}
        AbiType::Integer { .. } => {}
        AbiType::Boolean => {}
        AbiType::String { .. } => {}
    }
}
