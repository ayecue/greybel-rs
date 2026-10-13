use super::{LineIndex, Measure, offset};
use crate::lexer::{is_end_of_line, is_windows_newline};
use crate::position::{ColumnSettings, PositionEncoding};

pub(super) const UTF16: ColumnSettings = ColumnSettings {
    encoding: PositionEncoding::Utf16,
    tab_width: 1,
};

pub(super) const ROUND_TRIP_SOURCE: &str = "a\tb\r\né😀x\n\n\t😀\rend";

pub(super) fn round_trip_columns() -> [ColumnSettings; 3] {
    [
        UTF16,
        ColumnSettings::new(PositionEncoding::Utf8, 4),
        ColumnSettings::new(PositionEncoding::Utf32, 2),
    ]
}

pub(super) fn padded_sources() -> Vec<String> {
    let specials = ["\n", "\r\n", "\n\r", "\r", "\t", "é", "😀"];

    (0..=17)
        .flat_map(|padding| {
            specials.iter().map(move |special| {
                let run = "x".repeat(padding);

                format!("{run}{special}{run}{special}{run}")
            })
        })
        .collect()
}

pub(super) fn char_by_char(source: &str) -> LineIndex {
    let bytes = source.as_bytes();
    let mut line_starts = vec![0];
    let mut line_ends = Vec::new();
    let mut measures = Vec::new();
    let mut chars = source.char_indices().peekable();

    while let Some((index, c)) = chars.next() {
        let byte = bytes[index];

        if is_end_of_line(byte) {
            line_ends.push(offset(index));

            let next = bytes.get(index + 1).copied().unwrap_or(0);

            if is_windows_newline(byte, next) {
                chars.next();
                line_starts.push(offset(index + 2));
            } else {
                line_starts.push(offset(index + 1));
            }
        } else if c == '\t' {
            measures.push((offset(index), Measure::Tab));
        } else if !c.is_ascii() {
            measures.push((offset(index), Measure::NonAscii(c)));
        }
    }

    line_ends.push(offset(source.len()));

    LineIndex {
        line_starts,
        line_ends,
        measures,
    }
}
