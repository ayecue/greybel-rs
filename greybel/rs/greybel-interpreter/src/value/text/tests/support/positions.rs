use super::super::*;

pub const TEXT: &str = "aé😀b";
pub const ENCODINGS: [PositionEncoding; 3] = [
    PositionEncoding::Utf8,
    PositionEncoding::Utf16,
    PositionEncoding::Utf32,
];

pub struct Sample {
    text: Text,
    encoding: PositionEncoding,
}

impl Sample {
    pub fn new(text: &str, encoding: PositionEncoding) -> Self {
        Self {
            text: Text::new(text),
            encoding,
        }
    }

    pub fn len(&self) -> usize {
        self.text.unit_len(self.encoding)
    }

    pub fn slice(&self, range: Range<usize>) -> String {
        self.text.units(self.encoding).slice(range).to_string()
    }

    pub fn at(&self, index: usize) -> String {
        self.text.units(self.encoding).at(index).to_string()
    }

    pub fn repeat(&self, factor: f64) -> Option<String> {
        self.text
            .repeat_units(Number::new(factor), self.encoding)
            .map(|text| text.to_string())
    }
}

pub fn find(
    text: &str,
    search_text: &str,
    from: usize,
    encoding: PositionEncoding,
) -> Option<usize> {
    Text::new(text).units(encoding).find(search_text, from)
}
