use std::collections::HashSet;

use super::ConstraintTable;
use crate::error::DefinitionError;
use crate::model::{Definition, Field, Literal, TypeKind, TypeRef};

impl ConstraintTable<'_> {
    pub(super) fn verify_constants(&self) -> Result<(), DefinitionError> {
        let mut names = HashSet::new();

        for constant in &self.definition.constants {
            let name = &constant.name;

            if !Self::is_capitalized_snake_case(name) {
                return Err(DefinitionError::new(
                    constant.span,
                    format!("`{name}` is a constant's name in CAPITALIZED_SNAKE_CASE"),
                ));
            }

            if !names.insert(name.as_str()) {
                return Err(DefinitionError::new(
                    constant.span,
                    format!("the constant `{name}` is declared twice"),
                ));
            }

            if !Self::is_taking_literals(self.definition, &constant.ty) {
                return Err(DefinitionError::new(
                    constant.span,
                    format!(
                        "the constant `{name}` is of {}; a constant is a boolean, character, text, number or member of an enumeration",
                        constant.ty.keyword()
                    ),
                ));
            }

            if matches!(&constant.value, Literal::Name(other) if self.definition.constant(other).is_some())
            {
                return Err(DefinitionError::new(
                    constant.span,
                    format!("the constant `{name}` is a value, not another constant"),
                ));
            }

            self.definition
                .value_of(&constant.value, &constant.ty)
                .map_err(|error| {
                    DefinitionError::new(constant.span, format!("the constant `{name}` {error}"))
                })?;
        }

        Ok(())
    }

    pub(super) fn verify_default(&self, field: &Field) -> Result<(), DefinitionError> {
        let Some(default) = &field.default else {
            return Ok(());
        };
        let name = &field.name;

        if field.optional || field.flatten {
            return Err(DefinitionError::new(
                field.span,
                format!(
                    "the field `{name}` has a default, so it is neither optional nor flattened: it reads as its default"
                ),
            ));
        }

        if *default != Literal::Null
            && !Self::is_taking_literals(self.definition, field.ty.non_null())
        {
            return Err(DefinitionError::new(
                field.span,
                format!(
                    "the field `{name}` is of {}, whose default can only be null",
                    field.ty.keyword()
                ),
            ));
        }

        self.definition
            .value_of(default, &field.ty)
            .map(drop)
            .map_err(|error| {
                DefinitionError::new(field.span, format!("the default of `{name}` {error}"))
            })
    }

    pub(super) fn is_taking_literals(definition: &Definition, ty: &TypeRef) -> bool {
        match ty {
            TypeRef::Bool | TypeRef::Char | TypeRef::String(_) | TypeRef::Number(..) => true,
            TypeRef::Declared(name) => matches!(
                definition
                    .declaration(name)
                    .map(|declaration| &declaration.kind),
                Some(TypeKind::Enumeration(_))
            ),
            TypeRef::Bytes(_) | TypeRef::List(..) | TypeRef::Record(..) | TypeRef::Nullable(_) => {
                false
            }
        }
    }
}
