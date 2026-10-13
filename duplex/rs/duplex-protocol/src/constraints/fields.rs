use std::collections::HashSet;

use super::ConstraintTable;
use crate::error::DefinitionError;
use crate::model::{Field, TypeDeclaration, TypeKind, TypeRef};

impl<'a> ConstraintTable<'a> {
    pub(super) fn verify_fields(&self, fields: &[Field]) -> Result<(), DefinitionError> {
        let mut names = HashSet::new();

        self.verify_field_list(fields, &mut names, &mut Vec::new())
    }

    pub(super) fn verify_field_list(
        &self,
        fields: &'a [Field],
        names: &mut HashSet<&'a str>,
        flatten_path: &mut Vec<&'a str>,
    ) -> Result<(), DefinitionError> {
        for field in fields {
            if !Self::is_snake_case(&field.name) {
                return Err(DefinitionError::new(
                    field.span,
                    format!("`{}` is a field name in snake_case", field.name),
                ));
            }

            self.verify_type(&field.ty, field.span)?;
            self.verify_default(field)?;
            if !field.flatten {
                if !names.insert(&field.name) {
                    return Err(DefinitionError::new(
                        field.span,
                        format!("the field `{}` is declared twice", field.name),
                    ));
                }

                continue;
            }

            let flatten_target = match &field.ty {
                TypeRef::Declared(name) if !field.optional => self.types.get(name.as_str()),
                _ => None,
            };
            let Some(TypeDeclaration {
                name,
                kind: TypeKind::Struct(inner),
                ..
            }) = flatten_target
            else {
                return Err(DefinitionError::new(
                    field.span,
                    format!(
                        "the field `{}` flattens a struct, which is neither optional nor nullable",
                        field.name
                    ),
                ));
            };

            if flatten_path.contains(&name.as_str()) {
                return Err(DefinitionError::new(
                    field.span,
                    format!("`{name}` flattens itself"),
                ));
            }

            flatten_path.push(name);
            self.verify_field_list(inner, names, flatten_path)?;
            flatten_path.pop();
        }

        Ok(())
    }
}
