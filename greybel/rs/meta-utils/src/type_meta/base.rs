use std::fmt;

use greybel_symbols::annotations::ANY;
use greybel_symbols::types::{FUNCTION, LIST, MAP, NUMBER, STRING};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BaseType {
    Any,
    General,
    String,
    Function,
    Number,
    List,
    Map,
}

impl BaseType {
    pub const ALL: [Self; 7] = [
        Self::Any,
        Self::General,
        Self::String,
        Self::Function,
        Self::Number,
        Self::List,
        Self::Map,
    ];

    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Any => ANY,
            Self::General => "general",
            Self::String => STRING,
            Self::Function => FUNCTION,
            Self::Number => NUMBER,
            Self::List => LIST,
            Self::Map => MAP,
        }
    }

    #[must_use]
    pub fn from_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|base| base.as_str() == name)
    }
}

impl fmt::Display for BaseType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}
