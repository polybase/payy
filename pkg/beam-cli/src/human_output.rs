pub(crate) fn sanitize_control_chars(value: &str) -> String {
    value.chars().map(sanitize_control_char).collect()
}

pub(crate) fn sanitize_control_chars_trimmed(value: &str) -> String {
    sanitize_control_chars(value).trim().to_string()
}

pub(crate) fn normalize_human_name(value: &str) -> Option<String> {
    let value = sanitize_control_chars_trimmed(value);
    (!value.is_empty()).then_some(value)
}

pub(crate) fn escape_markdown_table_cell(value: &str) -> String {
    sanitize_control_chars(value)
        .replace('\\', "\\\\")
        .replace('|', "\\|")
}

fn sanitize_control_char(ch: char) -> char {
    match ch {
        '\n' | '\r' | '\t' => ' ',
        _ if ch.is_control() => '?',
        _ => ch,
    }
}
