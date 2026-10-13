use super::{Position, Range, Span};
use crate::lexer::{CARRIAGE_RETURN, LINE_FEED, TAB, is_end_of_line, is_windows_newline};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum PositionEncoding {
    Utf8,
    #[default]
    Utf16,
    Utf32,
}

impl PositionEncoding {
    #[must_use]
    pub fn count(self, text: &str) -> usize {
        match self {
            Self::Utf8 => text.len(),
            Self::Utf16 => str_indices::utf16::count(text),
            Self::Utf32 => str_indices::chars::count(text),
        }
    }

    #[must_use]
    pub fn char_len(self, c: char) -> usize {
        match self {
            Self::Utf8 => c.len_utf8(),
            Self::Utf16 => c.len_utf16(),
            Self::Utf32 => 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ColumnSettings {
    pub encoding: PositionEncoding,
    pub tab_width: u32,
}

impl Default for ColumnSettings {
    fn default() -> Self {
        Self {
            encoding: PositionEncoding::default(),
            tab_width: 1,
        }
    }
}

impl ColumnSettings {
    #[must_use]
    pub fn new(encoding: PositionEncoding, tab_width: u32) -> Self {
        Self {
            encoding,
            tab_width: tab_width.max(1),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Measure {
    Tab,
    NonAscii(char),
}

impl Measure {
    fn bytes(self) -> u32 {
        match self {
            Measure::Tab => 1,
            Measure::NonAscii(c) => offset(c.len_utf8()),
        }
    }

    fn adjustment(self, columns: ColumnSettings) -> i64 {
        match self {
            Measure::Tab => i64::from(columns.tab_width) - 1,
            Measure::NonAscii(c) => {
                let units = columns.encoding.char_len(c) as i64;

                units - c.len_utf8() as i64
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineIndex {
    line_starts: Vec<u32>,
    line_ends: Vec<u32>,
    measures: Vec<(u32, Measure)>,
}

fn offset(index: usize) -> u32 {
    u32::try_from(index).expect("sources are smaller than 4 GiB")
}

impl LineIndex {
    #[must_use]
    pub fn new(source: &str) -> Self {
        let bytes = source.as_bytes();
        let mut line_starts = Vec::with_capacity(bytes.len() / 32 + 1);

        line_starts.push(0);

        let mut line_ends = Vec::with_capacity(bytes.len() / 32 + 1);
        let mut measures = Vec::new();
        let mut index = 0;

        loop {
            index = Self::skip_plain(bytes, index);

            let Some(&byte) = bytes.get(index) else {
                break;
            };

            if !byte.is_ascii() {
                let c = source[index..]
                    .chars()
                    .next()
                    .expect("index is a char boundary");

                measures.push((offset(index), Measure::NonAscii(c)));
                index += c.len_utf8();
            } else if is_end_of_line(byte) {
                line_ends.push(offset(index));

                let next = bytes.get(index + 1).copied().unwrap_or(0);

                index += if is_windows_newline(byte, next) { 2 } else { 1 };
                line_starts.push(offset(index));
            } else {
                if byte == TAB {
                    measures.push((offset(index), Measure::Tab));
                }

                index += 1;
            }
        }

        line_ends.push(offset(source.len()));

        Self {
            line_starts,
            line_ends,
            measures,
        }
    }

    fn skip_plain(bytes: &[u8], mut index: usize) -> usize {
        const ONES: u64 = u64::from_ne_bytes([0x01; 8]);
        const HIGH: u64 = u64::from_ne_bytes([0x80; 8]);
        let has = |word: u64, byte: u8| {
            let diff = word ^ (ONES * u64::from(byte));

            diff.wrapping_sub(ONES) & !diff & HIGH != 0
        };

        while let Some(chunk) = bytes.get(index..index + 8) {
            let word = u64::from_ne_bytes(chunk.try_into().expect("a chunk has 8 bytes"));

            if word & HIGH != 0
                || has(word, LINE_FEED)
                || has(word, CARRIAGE_RETURN)
                || has(word, TAB)
            {
                break;
            }

            index += 8;
        }

        index
    }

    #[must_use]
    pub fn line_count(&self) -> u32 {
        offset(self.line_starts.len())
    }

    #[must_use]
    pub fn line(&self, offset: u32) -> u32 {
        self.line_starts.partition_point(|&start| start <= offset) as u32
    }

    #[must_use]
    pub fn line_start(&self, line: u32) -> Option<u32> {
        let index = usize::try_from(line).ok()?.checked_sub(1)?;

        self.line_starts.get(index).copied()
    }

    #[must_use]
    pub fn position(&self, offset: u32, columns: ColumnSettings) -> Position {
        let line = self.line(offset);
        let start = self.line_starts[line as usize - 1];
        let from = self.measures.partition_point(|&(at, _)| at < start);
        let to = self.measures.partition_point(|&(at, _)| at < offset);
        let adjustment: i64 = self.measures[from..to]
            .iter()
            .map(|&(_, measure)| measure.adjustment(columns))
            .sum();
        let column = i64::from(offset - start) + adjustment + 1;

        Position::new(line, u32::try_from(column).expect("columns are positive"))
    }

    #[must_use]
    pub fn offset(&self, position: Position, columns: ColumnSettings) -> Option<u32> {
        let index = usize::try_from(position.line).ok()?.checked_sub(1)?;
        let start = *self.line_starts.get(index)?;
        let end = self.line_ends[index];
        let target = i64::from(position.character.max(1));
        let from = self.measures.partition_point(|&(at, _)| at < start);
        let to = self.measures.partition_point(|&(at, _)| at < end);
        let mut at = start;
        let mut column = 1_i64;

        for &(measure_at, measure) in &self.measures[from..to] {
            let plain = i64::from(measure_at - at);

            if target < column + plain {
                return Some(at + u32::try_from(target - column).ok()?);
            }

            column += plain;

            let width = i64::from(measure.bytes()) + measure.adjustment(columns);

            if target < column + width {
                return Some(measure_at);
            }

            column += width;
            at = measure_at + measure.bytes();
        }

        let remainder = u32::try_from(target - column).ok()?;

        Some(at.saturating_add(remainder).min(end))
    }

    #[must_use]
    pub fn range(&self, span: Span, columns: ColumnSettings) -> Range {
        let end = if span.is_closed() {
            self.position(span.end, columns)
        } else {
            Position::default()
        };

        Range::new(self.position(span.start, columns), end)
    }
}

#[cfg(test)]
#[path = "tests/line_index.rs"]
mod tests;
