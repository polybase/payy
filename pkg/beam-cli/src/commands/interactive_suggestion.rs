use super::interactive_helper::completion_candidates;

pub(crate) fn completion_hint(line: &str, pos: usize) -> Option<String> {
    let prefix = line.get(..pos)?;
    if pos < line.len() || prefix.trim().is_empty() {
        return None;
    }

    let start = prefix
        .rfind(|ch: char| ch.is_whitespace())
        .map_or(0, |index| index + 1);
    let shared = shared_candidate_prefix(completion_candidates(line, pos))?;

    (shared.len() > prefix[start..].len()).then(|| shared[prefix[start..].len()..].to_string())
}

fn shared_candidate_prefix(candidates: Vec<String>) -> Option<String> {
    let mut candidates = candidates.into_iter();
    let first = candidates.next()?;

    Some(candidates.fold(first, |shared, candidate| {
        shared
            .chars()
            .zip(candidate.chars())
            .take_while(|(left, right)| left == right)
            .map(|(ch, _)| ch)
            .collect()
    }))
}
