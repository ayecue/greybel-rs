mod display;
mod table;

use std::hash::{Hash, Hasher};

pub use display::TypeDisplay;
pub(crate) use display::TypeNotation;
use meta_utils::{BaseType, Function};
pub(crate) use table::CachedApiMember;
pub use table::TypeTable;

use crate::index::index_type;
use crate::model::{ApiMethodId, DeclaredId, FunctionId, MapId};

index_type! {
    TypeId: "types";
}

impl TypeId {
    pub const ANY: Self = Self::from_u32(1);
    pub const NULL: Self = Self::from_u32(2);
    pub const NUMBER: Self = Self::from_u32(3);
    pub const STRING: Self = Self::from_u32(4);
    pub const UNKNOWN: Self = Self::from_u32(0);
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Type {
    Unknown,
    Any,
    Null,
    Number,
    String,
    List(TypeId),
    Map { key: TypeId, value: TypeId },
    Object(MapId),
    Function(FunctionRef),
    Class(&'static str),
    TypeParameter { name: Box<str>, bound: TypeId },
    Union(Box<[TypeId]>),
}

impl Type {
    #[must_use]
    pub fn base_type(&self) -> Option<BaseType> {
        Some(match self {
            Self::Number => BaseType::Number,
            Self::String => BaseType::String,
            Self::List(_) => BaseType::List,
            Self::Map { .. } | Self::Object(_) | Self::Class(_) => BaseType::Map,
            Self::Function(_) => BaseType::Function,
            Self::Unknown
            | Self::Any
            | Self::Null
            | Self::TypeParameter { .. }
            | Self::Union(_) => {
                return None;
            }
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FunctionRef {
    Any,
    Api(ApiFunction),
    ApiMethod(ApiMethodId),
    Body(FunctionId),
    Signature(DeclaredId),
}

#[derive(Debug, Clone, Copy)]
pub struct ApiFunction {
    pub origin: &'static str,
    pub name: &'static str,
    pub(crate) function: &'static Function,
}

impl ApiFunction {
    pub(crate) fn new(
        origin: &'static str,
        name: &'static str,
        function: &'static Function,
    ) -> Self {
        Self {
            origin,
            name,
            function,
        }
    }

    #[must_use]
    pub fn function(&self) -> &'static Function {
        self.function
    }
}

impl PartialEq for ApiFunction {
    fn eq(&self, other: &Self) -> bool {
        (self.origin, self.name) == (other.origin, other.name)
    }
}

impl Eq for ApiFunction {}

impl Hash for ApiFunction {
    fn hash<H: Hasher>(&self, state: &mut H) {
        (self.origin, self.name).hash(state);
    }
}
