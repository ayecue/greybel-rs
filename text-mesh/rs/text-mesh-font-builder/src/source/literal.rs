use std::fmt;

pub(crate) enum Literal {
    Float(f32),
    Char(char),
    Str(String),
}

impl Literal {
    pub(crate) const fn float(value: f32) -> Self {
        Self::Float(value)
    }

    pub(crate) const fn char(value: char) -> Self {
        Self::Char(value)
    }

    pub(crate) fn borrowed_str(value: &str) -> Self {
        Self::Str(value.to_owned())
    }
}

impl fmt::Display for Literal {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Float(value) if value.is_nan() => write!(f, "f32::NAN"),
            Self::Float(value) if value.is_infinite() && *value > 0.0 => write!(f, "f32::INFINITY"),
            Self::Float(value) if value.is_infinite() => write!(f, "f32::NEG_INFINITY"),
            Self::Float(value) => write!(f, "{value:?}_f32"),
            Self::Char(value) => write!(f, "{value:?}"),
            Self::Str(value) => write!(f, "::std::borrow::Cow::Borrowed({value:?})"),
        }
    }
}
