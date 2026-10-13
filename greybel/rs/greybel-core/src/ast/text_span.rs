use std::borrow::Cow;

use greybel_symbols::delimiters::{ESCAPED_QUOTE, QUOTE};

use crate::position::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TextSpan {
    span: Span,
    escaped: bool,
}

impl TextSpan {
    #[must_use]
    pub const fn new(span: Span) -> Self {
        Self {
            span,
            escaped: false,
        }
    }

    #[must_use]
    pub const fn with_escapes(span: Span) -> Self {
        Self {
            span,
            escaped: true,
        }
    }

    #[must_use]
    pub const fn span(&self) -> Span {
        self.span
    }

    #[must_use]
    pub const fn is_escaped(&self) -> bool {
        self.escaped
    }

    #[must_use]
    pub fn read<'s>(&self, source: &'s str) -> Cow<'s, str> {
        let text = self.span.bytes().map_or("", |bytes| &source[bytes]);

        if self.escaped {
            Cow::Owned(text.replace(ESCAPED_QUOTE, QUOTE))
        } else {
            Cow::Borrowed(text)
        }
    }
}
