use std::ops::Range;

use greybel_core::PositionEncoding;

use super::Text;
use crate::value::Number;

pub struct TextUnitView<'t> {
    text: &'t Text,
    ascii: bool,
    len: usize,
    encoding: PositionEncoding,
}

impl<'t> TextUnitView<'t> {
    pub(super) fn new(text: &'t Text, encoding: PositionEncoding) -> Self {
        let ascii = text.is_ascii();
        let len = if ascii {
            text.len()
        } else {
            encoding.count(text)
        };

        Self {
            text,
            ascii,
            len,
            encoding,
        }
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.len
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    #[must_use]
    pub fn slice(&self, range: Range<usize>) -> Text {
        if self.ascii {
            if let [byte] = self.text.as_bytes()[range.clone()] {
                return Text::of_char(char::from(byte));
            }

            return Text::with_ascii(&self.text[range], true);
        }

        Text::new(self.slice_unicode(range))
    }

    #[must_use]
    pub fn at(&self, index: usize) -> Text {
        self.slice(index..index + 1)
    }

    #[must_use]
    pub fn repeat(&self, factor: Number) -> Option<Text> {
        Some(self.repeat_to(factor.repeat_count(self.len)?))
    }

    #[must_use]
    pub fn repeat_to(&self, count: usize) -> Text {
        if count == 0 {
            return Text::with_ascii("", true);
        }

        let mut out = self.text.repeat(count / self.len);

        out.push_str(&self.slice(0..count % self.len));

        Text::new(out)
    }

    #[must_use]
    pub fn find(&self, text: &str, from: usize) -> Option<usize> {
        let start = self.byte_offset(from);
        let hit = start + self.text[start..].find(text)?;

        Some(self.units_before(hit))
    }

    fn byte_offset(&self, unit: usize) -> usize {
        if self.ascii {
            return unit.min(self.text.len());
        }

        let mut units = 0;

        for (at, c) in self.text.char_indices() {
            if units >= unit {
                return at;
            }

            units += self.encoding.char_len(c);
        }

        self.text.len()
    }

    #[must_use]
    pub fn units_before(&self, byte: usize) -> usize {
        if self.ascii {
            return byte;
        }

        self.encoding.count(&self.text[..byte])
    }

    fn slice_unicode(&self, range: Range<usize>) -> String {
        let text: &str = self.text;

        match self.encoding {
            PositionEncoding::Utf8 => String::from_utf8_lossy(&text.as_bytes()[range]).into_owned(),
            PositionEncoding::Utf16 => mono_string::utf16::substring(text, range).into_owned(),
            PositionEncoding::Utf32 => text.chars().skip(range.start).take(range.len()).collect(),
        }
    }
}

#[cfg(test)]
#[path = "tests/positions.rs"]
mod tests;
