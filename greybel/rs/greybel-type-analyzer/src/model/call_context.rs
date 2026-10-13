use super::FunctionId;
use crate::types::TypeId;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CallContext {
    pub function: FunctionId,
    pub receiver: Option<TypeId>,
    pub arguments: Vec<Option<TypeId>>,
}

impl CallContext {
    pub(crate) fn self_type(&self, has_self_parameter: bool) -> Option<TypeId> {
        match self.arguments.first() {
            Some(&first) if has_self_parameter => first,
            _ => self.receiver,
        }
    }

    pub(crate) fn has_unknown(&self) -> bool {
        self.receiver == Some(TypeId::UNKNOWN) || self.arguments.contains(&Some(TypeId::UNKNOWN))
    }
}
