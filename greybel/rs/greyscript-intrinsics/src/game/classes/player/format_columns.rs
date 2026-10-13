use greybel_interpreter::{Value, native};

use super::COLUMN_GAP;

#[native]
pub fn format_columns(#[name = "output_str"] output: &Value) -> String {
    match output.as_str() {
        Some(output) if !output.is_empty() => format_columns_text(output),
        _ => String::new(),
    }
}

/// Implements `ScriptsUtil.FormatColumnas`: split words on single spaces and pad each column to its widest word plus two spaces.
/// Rich-text tags do not count toward word width.
fn format_columns_text(text: &str) -> String {
    let width = |word: &str| mono_string::utf16::len(&without_tags(word));
    let text = text.replace("\\n", "\n");
    let rows: Vec<Vec<&str>> = text
        .split('\n')
        .map(|line| line.trim_end().split(' ').collect())
        .collect();
    let columns = rows.iter().map(Vec::len).max().unwrap_or(0);
    let mut widths = vec![0; columns];

    for row in &rows {
        for (column, word) in row.iter().enumerate() {
            widths[column] = widths[column].max(width(word));
        }
    }

    let mut out = String::new();

    for row in &rows {
        for (column, column_width) in widths.iter().enumerate() {
            if let Some(word) = row.get(column) {
                out.push_str(word);
                out.extend(std::iter::repeat_n(' ', column_width - width(word)));
            }

            out.push_str(COLUMN_GAP);
        }

        out.push('\n');
    }

    out.trim_end().to_owned()
}

/// Removes rich-text tags from `word` like `Regex.Replace(word, "<.*?>", "")`: each `<` through the next `>`.
fn without_tags(word: &str) -> String {
    let mut out = String::new();
    let mut remainder = word;

    while let Some(start) = remainder.find('<') {
        let Some(end) = remainder[start..].find('>') else {
            break;
        };

        out.push_str(&remainder[..start]);
        remainder = &remainder[start + end + 1..];
    }

    out.push_str(remainder);

    out
}

#[cfg(test)]
#[path = "tests/format_columns.rs"]
mod tests;
