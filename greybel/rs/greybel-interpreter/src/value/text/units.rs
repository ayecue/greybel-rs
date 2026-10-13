use std::rc::Rc;

use greybel_core::PositionEncoding;

use super::Text;

#[derive(Debug, Clone)]
pub enum EncodedText {
    Bytes(Text),
    Utf16(Rc<[u16]>),
    Chars(Rc<[char]>),
}

impl EncodedText {
    #[must_use]
    pub fn new(text: &Text, encoding: PositionEncoding) -> Self {
        if text.is_ascii() {
            return Self::Bytes(text.clone());
        }

        match encoding {
            PositionEncoding::Utf8 => Self::Bytes(text.clone()),
            PositionEncoding::Utf16 => Self::Utf16(text.encode_utf16().collect()),
            PositionEncoding::Utf32 => Self::Chars(text.chars().collect()),
        }
    }

    #[must_use]
    pub fn get(&self, index: usize) -> Option<Text> {
        let c = match self {
            Self::Bytes(text) => {
                let byte = *text.as_bytes().get(index)?;

                if byte.is_ascii() {
                    char::from(byte)
                } else {
                    char::REPLACEMENT_CHARACTER
                }
            }
            Self::Utf16(units) => {
                char::from_u32(u32::from(*units.get(index)?)).unwrap_or(char::REPLACEMENT_CHARACTER)
            }
            Self::Chars(chars) => *chars.get(index)?,
        };

        Some(Text::of_char(c))
    }
}
