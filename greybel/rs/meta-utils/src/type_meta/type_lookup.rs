use greybel_symbols::annotations::NEVER;
use greybel_symbols::types;

use super::BaseType;
use crate::signature::Signature;

#[derive(Debug, Clone, Copy)]
pub enum TypeLookup<'a> {
    Base(BaseType),
    Null,
    Never,
    Class(&'a Signature),
    Unknown,
}

impl TypeLookup<'_> {
    pub const NEVER: &'static str = NEVER;
    pub const NULL: &'static str = types::NULL;
}
