mod base;
mod parser;
mod payload;
mod type_lookup;

use std::fmt;
use std::str::FromStr;

pub use base::BaseType;
pub use parser::MAX_TYPE_DEPTH;
pub use type_lookup::TypeLookup;

use crate::error::TypeParseError;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TypeMeta {
    Name(String),
    List(Box<TypeMeta>),
    Map {
        key: Box<TypeMeta>,
        value: Box<TypeMeta>,
    },
}

impl TypeMeta {
    #[must_use]
    pub fn any() -> Self {
        Self::Name(BaseType::Any.as_str().to_owned())
    }

    #[must_use]
    pub fn from_name(name: impl Into<String>) -> Self {
        let name = name.into();

        match BaseType::from_name(&name) {
            Some(BaseType::List) => Self::List(Box::new(Self::any())),
            Some(BaseType::Map) => Self::Map {
                key: Box::new(Self::any()),
                value: Box::new(Self::any()),
            },
            _ => Self::Name(name),
        }
    }

    #[must_use]
    pub fn name(&self) -> &str {
        match self {
            Self::Name(name) => name,
            Self::List(_) => BaseType::List.as_str(),
            Self::Map { .. } => BaseType::Map.as_str(),
        }
    }

    #[must_use]
    pub fn is(&self, base: BaseType) -> bool {
        self.name() == base.as_str()
    }

    #[must_use]
    pub fn key_type(&self) -> Option<&Self> {
        match self {
            Self::Map { key, .. } => Some(key),
            _ => None,
        }
    }

    #[must_use]
    pub fn value_type(&self) -> Option<&Self> {
        match self {
            Self::List(value) | Self::Map { value, .. } => Some(value),
            Self::Name(_) => None,
        }
    }
}

impl FromStr for TypeMeta {
    type Err = TypeParseError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        parser::parse(text)
    }
}

impl fmt::Display for TypeMeta {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Name(name) => f.write_str(name),
            Self::List(value) => write!(f, "list<{value}>"),
            Self::Map { key, value } => write!(f, "map<{key},{value}>"),
        }
    }
}
