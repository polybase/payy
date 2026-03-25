use std::fs;

use contextful::ResultContextExt;

use crate::error::{Result, XTaskError};

use super::runner::{CircuitConfig, HashUpdateKind, VerificationKeyHash};

pub(super) fn apply_hash_updates(
    circuit: &CircuitConfig,
    hash: &VerificationKeyHash,
) -> Result<()> {
    for update in &circuit.hash_updates {
        let target_path = circuit.package_dir.join(&update.path);
        let source = fs::read_to_string(&target_path)
            .with_context(|| format!("read {}", target_path.display()))?;

        let updated = match update.kind {
            HashUpdateKind::GlobalField => {
                replace_global_field(&source, &update.name, &hash.as_field).ok_or_else(|| {
                    XTaskError::NoirHashUpdateNotFound {
                        path: target_path.clone(),
                        name: update.name.clone(),
                    }
                })?
            }
            HashUpdateKind::ConstString => {
                replace_const_string(&source, &update.name, &hash.as_hex).ok_or_else(|| {
                    XTaskError::NoirHashUpdateNotFound {
                        path: target_path.clone(),
                        name: update.name.clone(),
                    }
                })?
            }
        };

        fs::write(&target_path, updated)
            .with_context(|| format!("write {}", target_path.display()))?;
    }

    Ok(())
}

fn replace_global_field(source: &str, name: &str, value: &str) -> Option<String> {
    let marker = format!("global {name}: Field =");
    let mut replacements = 0usize;
    let mut output = String::with_capacity(source.len() + value.len() + 32);

    for line in source.split_inclusive('\n') {
        let has_newline = line.ends_with('\n');
        let line_body = if has_newline {
            &line[..line.len().saturating_sub(1)]
        } else {
            line
        };
        let trimmed = line_body.trim_start();

        if trimmed.starts_with(&marker) {
            replacements += 1;
            let indent_len = line_body.len().saturating_sub(trimmed.len());
            output.push_str(&line_body[..indent_len]);
            output.push_str("global ");
            output.push_str(name);
            output.push_str(": Field = ");
            output.push_str(value);
            output.push(';');
            if has_newline {
                output.push('\n');
            }
            continue;
        }

        output.push_str(line);
    }

    if replacements == 1 {
        Some(output)
    } else {
        None
    }
}

fn replace_const_string(source: &str, name: &str, value: &str) -> Option<String> {
    let marker = format!("const {name}");
    let marker_pos = source.find(&marker)?;
    let value_start = source[marker_pos..].find('"')? + marker_pos;
    let value_end = source[value_start + 1..].find('"')? + value_start + 1;

    let mut output = String::with_capacity(source.len() + value.len() + 8);
    output.push_str(&source[..value_start + 1]);
    output.push_str(value);
    output.push_str(&source[value_end..]);
    Some(output)
}
