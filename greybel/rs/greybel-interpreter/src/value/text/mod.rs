mod positions;
mod units;

use std::fmt;
use std::ops::Deref;
use std::rc::Rc;

use greybel_core::PositionEncoding;
pub use positions::TextUnitView;
use thin_str::ThinStr;
pub use units::EncodedText;

use super::Number;

#[derive(Debug, Clone)]
pub struct Text(ThinStr<bool>);

thread_local! {
    static ASCII_CHARACTERS: [Text; 128] = std::array::from_fn(|byte| {
        let byte = u8::try_from(byte).expect("an ASCII byte");
        Text::with_ascii(char::from(byte).encode_utf8(&mut [0; 4]), true)
    });
}

impl Text {
    #[must_use]
    pub fn new(text: impl AsRef<str>) -> Self {
        let text = text.as_ref();

        Self::with_ascii(text, text.is_ascii())
    }

    pub(crate) fn with_ascii(text: &str, ascii: bool) -> Self {
        debug_assert_eq!(text.is_ascii(), ascii);

        Self(ThinStr::new(ascii, text))
    }

    pub(crate) fn concat(parts: &[&str], ascii: bool) -> Self {
        debug_assert_eq!(parts.iter().all(|part| part.is_ascii()), ascii);

        Self(ThinStr::concat(ascii, parts))
    }

    #[must_use]
    pub fn of_char(c: char) -> Self {
        match u8::try_from(c) {
            Ok(byte) if byte.is_ascii() => {
                ASCII_CHARACTERS.with(|characters| characters[usize::from(byte)].clone())
            }
            _ => Self::with_ascii(c.encode_utf8(&mut [0; 4]), false),
        }
    }

    #[must_use]
    pub fn is_ascii(&self) -> bool {
        self.0.header()
    }

    #[must_use]
    pub fn ptr_eq(&self, other: &Self) -> bool {
        ThinStr::ptr_eq(&self.0, &other.0)
    }

    #[must_use]
    pub fn units(&self, encoding: PositionEncoding) -> TextUnitView<'_> {
        TextUnitView::new(self, encoding)
    }

    #[must_use]
    pub fn unit_len(&self, encoding: PositionEncoding) -> usize {
        self.units(encoding).len()
    }

    #[must_use]
    pub fn repeat_units(&self, factor: Number, encoding: PositionEncoding) -> Option<Self> {
        self.units(encoding).repeat(factor)
    }
}

impl Deref for Text {
    type Target = str;

    fn deref(&self) -> &str {
        self.0.as_str()
    }
}

impl PartialEq for Text {
    fn eq(&self, other: &Self) -> bool {
        self.ptr_eq(other) || **self == **other
    }
}

impl Eq for Text {}

impl fmt::Display for Text {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self)
    }
}

impl From<&str> for Text {
    fn from(text: &str) -> Self {
        Self::new(text)
    }
}

impl From<String> for Text {
    fn from(text: String) -> Self {
        Self::new(text)
    }
}

impl From<Rc<str>> for Text {
    fn from(text: Rc<str>) -> Self {
        Self::new(&*text)
    }
}
