use super::ConstraintTable;
use crate::error::DefinitionError;
use crate::model::{Payload, TypeDeclaration, TypeKind, TypeRef};

impl<'a> ConstraintTable<'a> {
    pub(super) fn verify_finite(
        &self,
        declaration: &'a TypeDeclaration,
        path: &mut Vec<&'a str>,
    ) -> Result<(), DefinitionError> {
        if path.contains(&declaration.name.as_str()) {
            return Err(DefinitionError::new(
                declaration.span,
                format!(
                    "`{}` contains itself; put it in a list or a record",
                    declaration.name
                ),
            ));
        }

        path.push(&declaration.name);

        let fields: Vec<&TypeRef> = match &declaration.kind {
            TypeKind::Struct(fields) => fields.iter().map(|field| &field.ty).collect(),
            TypeKind::Union(variants) => variants
                .iter()
                .flat_map(|variant| match &variant.payload {
                    Payload::Unit => Vec::new(),
                    Payload::Value(ty) => vec![ty],
                    Payload::Fields(fields) => fields.iter().map(|field| &field.ty).collect(),
                })
                .collect(),
            TypeKind::Enumeration(_) => Vec::new(),
        };

        for ty in fields {
            if let TypeRef::Declared(name) = ty.non_null()
                && let Some(inner) = self.types.get(name.as_str())
            {
                self.verify_finite(inner, path)?;
            }
        }

        path.pop();

        Ok(())
    }
}
