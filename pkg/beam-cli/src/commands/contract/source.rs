use std::collections::BTreeMap;

use sourcify_interface::SourceFile;

use super::error::{Error, Result};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum SourceMatchKind {
    Basename,
    Exact,
}

#[derive(Clone, Debug)]
pub(super) struct MatchedSource<'a> {
    pub(super) content: &'a str,
    pub(super) kind: SourceMatchKind,
    pub(super) path: String,
}

pub(super) fn require_sources(
    sources: Option<&BTreeMap<String, SourceFile>>,
) -> Result<&BTreeMap<String, SourceFile>> {
    let Some(sources) = sources else {
        return Err(Error::SourcifyMalformedResponse {
            reason: "sources field is missing".to_owned(),
        });
    };
    if sources.is_empty() {
        return Err(Error::SourcifyMalformedResponse {
            reason: "sources field is empty".to_owned(),
        });
    }

    Ok(sources)
}

pub(super) fn source_paths(sources: &BTreeMap<String, SourceFile>) -> Vec<String> {
    sources.keys().cloned().collect()
}

pub(super) fn match_source_path<'a>(
    sources: &'a BTreeMap<String, SourceFile>,
    requested_path: &str,
) -> Result<MatchedSource<'a>> {
    if let Some(source) = sources.get(requested_path) {
        return Ok(MatchedSource {
            content: &source.content,
            kind: SourceMatchKind::Exact,
            path: requested_path.to_owned(),
        });
    }

    let matches = sources
        .iter()
        .filter(|(path, _)| basename(path) == requested_path)
        .collect::<Vec<_>>();

    match matches.as_slice() {
        [] => Err(Error::SourcePathNotFound {
            path: requested_path.to_owned(),
        }),
        [(path, source)] => Ok(MatchedSource {
            content: &source.content,
            kind: SourceMatchKind::Basename,
            path: (*path).clone(),
        }),
        _ => Err(Error::SourcePathAmbiguous {
            matches: matches.iter().map(|(path, _)| (*path).clone()).collect(),
            path: requested_path.to_owned(),
        }),
    }
}

impl SourceMatchKind {
    pub(super) fn as_str(&self) -> &'static str {
        match self {
            Self::Basename => "basename",
            Self::Exact => "exact",
        }
    }
}

fn basename(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}
