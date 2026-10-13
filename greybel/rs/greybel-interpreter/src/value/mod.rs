mod class_id;
mod display;
mod equal;
mod function;
mod hash;
mod hash_code;
mod heap;
mod key;
mod number;
mod ops;
mod order;
mod text;

pub use class_id::CLASS_ID;
pub use display::ValueDisplay;
pub use function::{Closure, FunctionBody, Param};
use greybel_core::{Ast, LiteralValue};
use greybel_symbols::types;
pub use hash::KeyHash;
pub use heap::{Heap, HeapRef, ItemsMut, MapLabel};
pub use key::Key;
use mono_dictionary::Dictionary;
pub use number::Number;
pub use text::{EncodedText, Text, TextUnitView};

pub type ValueMap = Dictionary<Value, Value>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct List(pub(crate) u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Map(pub(crate) u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Function(pub(crate) u32);

#[derive(Debug, Clone, Default)]
pub enum Value {
    #[default]
    Null,
    Number(Number),
    String(Text),
    List(List),
    Map(Map),
    Function(Function),
}

impl Value {
    pub const FALSE: Self = Self::Number(Number::ZERO);
    pub const TRUE: Self = Self::Number(Number::ONE);

    #[must_use]
    pub fn from_string(text: impl Into<Text>) -> Self {
        Self::String(text.into())
    }

    pub(crate) fn from_literal(ast: &Ast, value: &LiteralValue, negated: bool) -> Self {
        let number = match value {
            LiteralValue::String(text) => return Self::from_string(ast.read(*text).as_ref()),
            LiteralValue::Number(number) => Number::new(*number),
            LiteralValue::Boolean(boolean) => Number::from(*boolean),
            LiteralValue::Nil => return Self::Null,
        };

        Self::Number(if negated { -number } else { number })
    }

    #[must_use]
    pub fn type_name(&self) -> &'static str {
        match self {
            Self::Null => types::NULL,
            Self::Number(_) => types::NUMBER,
            Self::String(_) => types::STRING,
            Self::List(_) => types::LIST,
            Self::Map(_) => types::MAP,
            Self::Function(_) => types::FUNCTION,
        }
    }

    #[must_use]
    pub fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }

    #[must_use]
    pub fn as_str(&self) -> Option<&str> {
        self.as_text().map(|text| &**text)
    }

    #[must_use]
    pub fn as_text(&self) -> Option<&Text> {
        match self {
            Self::String(text) => Some(text),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_number(&self) -> Option<f64> {
        match self {
            Self::Number(number) => Some(number.get()),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_int(&self) -> Option<i32> {
        match self {
            Self::Number(number) => Some(number.to_int()),
            _ => None,
        }
    }

    #[must_use]
    pub fn to_number(&self) -> f64 {
        match self {
            Self::Number(number) => number.get(),
            Self::String(_) | Self::List(_) | Self::Map(_) => 0.0,
            Self::Null | Self::Function(_) => f64::NAN,
        }
    }

    #[must_use]
    pub fn to_int(&self) -> i32 {
        match self {
            Self::Number(number) => number.to_int(),
            _ => 0,
        }
    }

    #[must_use]
    pub fn is_identical(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Null, Self::Null) => true,
            (Self::Number(a), Self::Number(b)) => a.is_identical(*b),
            (Self::String(a), Self::String(b)) => a == b,
            (Self::List(a), Self::List(b)) => a == b,
            (Self::Map(a), Self::Map(b)) => a == b,
            (Self::Function(a), Self::Function(b)) => a == b,
            _ => false,
        }
    }
}

impl From<bool> for Value {
    fn from(value: bool) -> Self {
        if value { Self::TRUE } else { Self::FALSE }
    }
}

impl From<f64> for Value {
    fn from(value: f64) -> Self {
        Self::Number(Number::new(value))
    }
}

impl From<Number> for Value {
    fn from(value: Number) -> Self {
        Self::Number(value)
    }
}

impl From<&str> for Value {
    fn from(value: &str) -> Self {
        Self::String(value.into())
    }
}

impl From<String> for Value {
    fn from(value: String) -> Self {
        Self::String(value.into())
    }
}

impl From<Text> for Value {
    fn from(value: Text) -> Self {
        Self::String(value)
    }
}
