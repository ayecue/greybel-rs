use greybel_core::{Span, TextSpan};

use super::super::*;

pub fn string() -> Literal {
    Literal {
        value: LiteralValue::String(TextSpan::new(Span::new(1, 5))),
        negated: false,
    }
}

pub fn number() -> Literal {
    Literal {
        value: LiteralValue::Number(12.0),
        negated: false,
    }
}
