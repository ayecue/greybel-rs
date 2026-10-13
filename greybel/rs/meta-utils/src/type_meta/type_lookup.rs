use greybel_symbols::types;

use super::BaseType;
use crate::signature::Signature;

#[derive(Debug, Clone, Copy)]
pub enum TypeLookup<'a> {
    Base(BaseType),
    Null,
    Class(&'a Signature),
    Unknown,
}

impl TypeLookup<'_> {
    pub const NULL: &'static str = types::NULL;
}
