// lint-long-file-override allow-max-lines=240
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use contextful::ResultContextExt;
use serde_json::Value;

use crate::error::{Result, XTaskError};

use crate::lint::steps::StepResult;

const LOCALES_DIR: &str = "app/packages/payy/src/i18n/locales";
const REFERENCE_LOCALE: &str = "en";

pub fn run(repo_root: &Path) -> Result<StepResult> {
    let start = Instant::now();
    let locales_dir = repo_root.join(LOCALES_DIR);

    if !locales_dir.exists() {
        return Ok(StepResult::skipped(
            "I18n consistency",
            format!("Locales directory not found at {}", locales_dir.display()),
            start.elapsed(),
        ));
    }

    let locale_files = collect_locale_files(&locales_dir)?;
    if locale_files.is_empty() {
        return Ok(StepResult::skipped(
            "I18n consistency",
            format!("No locale files found in {}", locales_dir.display()),
            start.elapsed(),
        ));
    }

    let reference = match locale_files
        .iter()
        .find(|locale| locale.code == REFERENCE_LOCALE)
    {
        Some(locale) => locale,
        None => {
            let message = format!(
                "Reference locale {REFERENCE_LOCALE}.json not found in {}",
                locales_dir.display()
            );
            return Ok(StepResult::failed(
                "I18n consistency",
                message,
                start.elapsed(),
            ));
        }
    };

    let reference_keys = match extract_keys(reference) {
        Ok(keys) => keys,
        Err(message) => {
            return Ok(StepResult::failed(
                "I18n consistency",
                format!("en.json: {message}"),
                start.elapsed(),
            ));
        }
    };
    let mut mismatches = Vec::new();
    let mut mismatch_count = 0usize;

    for locale in &locale_files {
        if locale.code == REFERENCE_LOCALE {
            continue;
        }

        let keys = match extract_keys(locale) {
            Ok(keys) => keys,
            Err(message) => {
                mismatches.push(render_invalid_locale(
                    locale.path.strip_prefix(repo_root).unwrap_or(&locale.path),
                    &message,
                ));
                mismatch_count += 1;
                continue;
            }
        };
        let missing = reference_keys
            .difference(&keys)
            .cloned()
            .collect::<Vec<_>>();
        let extra = keys
            .difference(&reference_keys)
            .cloned()
            .collect::<Vec<_>>();

        if missing.is_empty() && extra.is_empty() {
            continue;
        }

        mismatch_count += missing.len() + extra.len();
        mismatches.push(render_mismatch(
            locale.path.strip_prefix(repo_root).unwrap_or(&locale.path),
            &missing,
            &extra,
        ));
    }

    if mismatches.is_empty() {
        return Ok(StepResult::success(
            "I18n consistency",
            format!(
                "Checked {} locale file(s); {} keys verified",
                locale_files.len(),
                reference_keys.len()
            ),
            start.elapsed(),
        ));
    }

    let summary = format!(
        "{mismatch_count} translation key mismatch(es) detected across {} locale(s)",
        mismatches.len()
    );

    Ok(
        StepResult::failed("I18n consistency", summary, start.elapsed())
            .with_extra_output(mismatches),
    )
}

struct LocaleFile {
    code: String,
    path: PathBuf,
    value: Value,
}

fn collect_locale_files(locales_dir: &Path) -> Result<Vec<LocaleFile>> {
    let mut locales = BTreeMap::new();
    let entries =
        fs::read_dir(locales_dir).with_context(|| format!("list {}", locales_dir.display()))?;

    for entry in entries {
        let entry = entry.with_context(|| format!("iterate {}", locales_dir.display()))?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }

        let code = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .ok_or(XTaskError::NonUtf8Path { path: path.clone() })?;

        let content =
            fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
        let value = serde_json::from_str::<Value>(&content)
            .with_context(|| format!("parse {}", path.display()))?;

        locales.insert(
            code.to_string(),
            LocaleFile {
                code: code.to_string(),
                path,
                value,
            },
        );
    }

    Ok(locales.into_values().collect())
}

fn extract_keys(locale: &LocaleFile) -> std::result::Result<BTreeSet<String>, String> {
    if !locale.value.is_object() {
        return Err("root value must be a JSON object".to_string());
    }

    let mut keys = BTreeSet::new();
    collect_paths(&locale.value, &mut keys, "");
    Ok(keys)
}

fn collect_paths(value: &Value, keys: &mut BTreeSet<String>, prefix: &str) {
    match value {
        Value::Object(map) => {
            if map.is_empty() && !prefix.is_empty() {
                keys.insert(prefix.to_string());
            }

            for (child_key, child_value) in map {
                let next = if prefix.is_empty() {
                    child_key.clone()
                } else {
                    format!("{prefix}.{child_key}")
                };
                collect_paths(child_value, keys, &next);
            }
        }
        _ => {
            if !prefix.is_empty() {
                keys.insert(prefix.to_string());
            }
        }
    }
}

fn render_mismatch(path: &Path, missing: &[String], extra: &[String]) -> String {
    let mut lines = Vec::new();
    lines.push(format!("{}:", path.display()));

    if !missing.is_empty() {
        lines.push("  missing keys:".to_string());
        for key in missing {
            lines.push(format!("    - {key}"));
        }
    }

    if !extra.is_empty() {
        lines.push("  extra keys:".to_string());
        for key in extra {
            lines.push(format!("    - {key}"));
        }
    }

    lines.join("\n")
}

fn render_invalid_locale(path: &Path, message: &str) -> String {
    format!("{}:\n  invalid locale structure: {message}", path.display())
}
