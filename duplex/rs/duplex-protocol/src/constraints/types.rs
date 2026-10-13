use super::ConstraintTable;
use crate::error::DefinitionError;
use crate::model::{Span, TypeKind, TypeRef};

impl ConstraintTable<'_> {
    pub(super) fn verify_type(&self, ty: &TypeRef, span: Span) -> Result<(), DefinitionError> {
        Self::verify_requirements(ty, span)?;

        match ty {
            TypeRef::Bool
            | TypeRef::Char
            | TypeRef::String(_)
            | TypeRef::Bytes(_)
            | TypeRef::Number(..) => Ok(()),
            TypeRef::List(item, _) | TypeRef::Record(item, _) => self.verify_type(item, span),
            TypeRef::Nullable(inner) => self.verify_type(inner, span),
            TypeRef::Declared(name) if self.types.contains_key(name.as_str()) => Ok(()),
            TypeRef::Declared(name) => Err(DefinitionError::new(
                span,
                format!("no type `{name}` is declared"),
            )),
        }
    }

    pub(super) fn verify_failure(&self, ty: &TypeRef, span: Span) -> Result<(), DefinitionError> {
        self.verify_declared_failure(ty, span, "a response fails with")
    }

    pub(super) fn verify_declared_failure(
        &self,
        ty: &TypeRef,
        span: Span,
        what: &str,
    ) -> Result<(), DefinitionError> {
        self.verify_type(ty, span)?;

        let declaration = match ty {
            TypeRef::Declared(name) => self.types.get(name.as_str()),
            _ => None,
        };

        match declaration.map(|declaration| &declaration.kind) {
            Some(TypeKind::Enumeration(_) | TypeKind::Union(_)) => Ok(()),
            _ => Err(DefinitionError::new(
                span,
                format!("{what} a declared enumeration or union"),
            )),
        }
    }
}
