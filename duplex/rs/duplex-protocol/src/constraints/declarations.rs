use std::collections::HashSet;

use super::ConstraintTable;
use crate::error::DefinitionError;
use crate::model::{Member, Payload, TypeDeclaration, TypeKind};

impl ConstraintTable<'_> {
    pub(super) fn verify_declaration(
        &self,
        declaration: &TypeDeclaration,
    ) -> Result<(), DefinitionError> {
        match &declaration.kind {
            TypeKind::Struct(fields) => self.verify_fields(fields),
            TypeKind::Union(variants) => {
                if variants.is_empty() {
                    return Err(DefinitionError::new(
                        declaration.span,
                        format!("the union `{}` has variants", declaration.name),
                    ));
                }

                let mut names = HashSet::new();

                for variant in variants {
                    Self::verify_member_name(&mut names, &variant.name, variant.span)?;

                    match &variant.payload {
                        Payload::Unit => {}
                        Payload::Value(ty) => self.verify_type(ty, variant.span)?,
                        Payload::Fields(fields) => self.verify_fields(fields)?,
                    }
                }

                Ok(())
            }
            TypeKind::Enumeration(members) => {
                if members.is_empty() {
                    return Err(DefinitionError::new(
                        declaration.span,
                        format!("the enumeration `{}` has members", declaration.name),
                    ));
                }

                Self::verify_member_values(declaration, members)?;

                let mut names = HashSet::new();

                for member in members {
                    Self::verify_member_name(&mut names, &member.name, member.span)?;
                }

                Ok(())
            }
        }
    }

    pub(super) fn verify_member_values(
        declaration: &TypeDeclaration,
        members: &[Member],
    ) -> Result<(), DefinitionError> {
        if members.iter().any(|member| member.value.is_some())
            && let Some(member) = members.iter().find(|member| member.value.is_none())
        {
            return Err(DefinitionError::new(
                member.span,
                format!(
                    "the enumeration `{}` gives every member a value or none",
                    declaration.name
                ),
            ));
        }

        let mut values = HashSet::new();

        for member in members {
            if let Some(value) = member.value
                && !values.insert(value)
            {
                return Err(DefinitionError::new(
                    member.span,
                    format!("the value {value} is given to another member"),
                ));
            }
        }

        Ok(())
    }
}
