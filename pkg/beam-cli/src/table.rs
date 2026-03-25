use crate::human_output::{escape_markdown_table_cell, sanitize_control_chars};

pub fn render_table(headers: &[&str], rows: &[Vec<String>]) -> String {
    let headers = headers
        .iter()
        .map(|header| sanitize_control_chars(header))
        .collect::<Vec<_>>();
    let rows = rows
        .iter()
        .map(|row| {
            row.iter()
                .map(|value| sanitize_control_chars(value))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let mut widths = headers.iter().map(String::len).collect::<Vec<_>>();

    for row in &rows {
        for (index, value) in row.iter().enumerate() {
            widths[index] = widths[index].max(value.len());
        }
    }

    let mut lines = Vec::with_capacity(rows.len() + 2);
    lines.push(render_row(&headers, &widths));
    lines.push(
        widths
            .iter()
            .map(|width| "-".repeat(*width))
            .collect::<Vec<_>>()
            .join("  "),
    );

    for row in &rows {
        lines.push(render_row(row, &widths));
    }

    lines.join("\n")
}

pub fn render_markdown_table(headers: &[&str], rows: &[Vec<String>]) -> String {
    let mut lines = Vec::with_capacity(rows.len() + 2);
    lines.push(format!(
        "| {} |",
        headers
            .iter()
            .map(|header| escape_markdown_table_cell(header))
            .collect::<Vec<_>>()
            .join(" | ")
    ));
    lines.push(format!(
        "| {} |",
        headers
            .iter()
            .map(|_| "---")
            .collect::<Vec<_>>()
            .join(" | ")
    ));

    for row in rows {
        lines.push(format!(
            "| {} |",
            row.iter()
                .map(|value| escape_markdown_table_cell(value))
                .collect::<Vec<_>>()
                .join(" | ")
        ));
    }

    lines.join("\n")
}

fn render_row(values: &[String], widths: &[usize]) -> String {
    values
        .iter()
        .enumerate()
        .map(|(index, value)| format!("{value:<width$}", width = widths[index]))
        .collect::<Vec<_>>()
        .join("  ")
}
