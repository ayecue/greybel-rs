use std::collections::HashSet;

use super::{Comparison, Mode};
use crate::error::DefinitionError;
use crate::model::{Field, Member, Payload, Span, TypeKind, TypeRef, Variant};

impl<'a> Comparison<'a> {
    pub(super) fn by_name(&mut self) -> Result<(), DefinitionError> {
        let (old_definition, new_definition) = (self.older, self.newer);
        let matched_old_indices: HashSet<usize> = self.pairs.values().copied().collect();

        for (new_index, declaration) in new_definition.types.iter().enumerate() {
            if self.pairs.contains_key(&new_index) {
                continue;
            }

            let Some(old_index) = self.old_index(&declaration.name) else {
                continue;
            };

            if matched_old_indices.contains(&old_index) {
                continue;
            }

            self.compare_type(old_index, new_index, Mode::Append)?;
        }

        if new_definition.protocol.is_none()
            && let Some(old_extra) = old_definition
                .types
                .iter()
                .find(|old| new_definition.declaration(&old.name).is_none())
        {
            return Err(DefinitionError::new(
                Span::default(),
                format!("removes the type `{}`", old_extra.name),
            ));
        }

        Ok(())
    }

    pub(super) fn ty(
        &mut self,
        old: &TypeRef,
        new: &TypeRef,
        mode: Mode,
        span: Span,
        what: &str,
    ) -> Result<(), DefinitionError> {
        match (old, new) {
            (TypeRef::Bool, TypeRef::Bool)
            | (TypeRef::Char, TypeRef::Char)
            | (TypeRef::String(_), TypeRef::String(_))
            | (TypeRef::Bytes(_), TypeRef::Bytes(_)) => Ok(()),
            (TypeRef::Number(old, _), TypeRef::Number(new, _)) if old == new => Ok(()),
            (TypeRef::List(old, _), TypeRef::List(new, _))
            | (TypeRef::Record(old, _), TypeRef::Record(new, _))
            | (TypeRef::Nullable(old), TypeRef::Nullable(new)) => {
                self.ty(old, new, mode, span, what)
            }
            (TypeRef::Declared(old), TypeRef::Declared(new)) => {
                match (self.old_index(old), self.new_index(new)) {
                    (Some(old), Some(new)) => self.compare_type(old, new, mode),
                    _ => Err(DefinitionError::new(span, format!("changes {what}"))),
                }
            }
            _ => Err(DefinitionError::new(
                span,
                format!("changes the type of {what}"),
            )),
        }
    }

    pub(super) fn compare_type(
        &mut self,
        old_index: usize,
        new_index: usize,
        mode: Mode,
    ) -> Result<(), DefinitionError> {
        let (old_definition, new_definition) = (self.older, self.newer);
        let old = &old_definition.types[old_index];
        let new = &new_definition.types[new_index];

        match self.pairs.get(&new_index) {
            Some(&matched_old_index) if matched_old_index != old_index => {
                return Err(DefinitionError::new(
                    new.span,
                    format!(
                        "`{}` takes the place of both `{}` and `{}`",
                        new.name, old_definition.types[matched_old_index].name, old.name
                    ),
                ));
            }
            _ => {}
        }

        self.pairs.insert(new_index, old_index);
        if !self.compared.insert((new_index, mode)) {
            return Ok(());
        }

        match (&old.kind, &new.kind) {
            (TypeKind::Struct(old_fields), TypeKind::Struct(new_fields)) => {
                self.fields(old_fields, new_fields, mode, new.span)
            }
            (TypeKind::Union(old_variants), TypeKind::Union(new_variants)) => {
                self.variants(old_variants, new_variants, mode, new.span)
            }
            (TypeKind::Enumeration(old_members), TypeKind::Enumeration(new_members)) => {
                self.members(old_members, new_members, mode, new.span)
            }
            _ => Err(DefinitionError::new(
                new.span,
                format!(
                    "changes whether `{}` is a struct, a union or an enumeration",
                    new.name
                ),
            )),
        }
    }

    pub(super) fn fields(
        &mut self,
        old: &[Field],
        new: &[Field],
        mode: Mode,
        span: Span,
    ) -> Result<(), DefinitionError> {
        if let Some(old_extra) = old.get(new.len()) {
            return Err(DefinitionError::new(
                span,
                format!("removes the field `{}`", old_extra.name),
            ));
        }

        for (old_field, new_field) in old.iter().zip(new) {
            if old_field.optional != new_field.optional {
                return Err(DefinitionError::new(
                    new_field.span,
                    format!("changes whether `{}` is optional", new_field.name),
                ));
            }

            if old_field.flatten != new_field.flatten {
                return Err(DefinitionError::new(
                    new_field.span,
                    format!("changes whether `{}` is flattened", new_field.name),
                ));
            }

            let what = format!("`{}`", new_field.name);

            self.ty(&old_field.ty, &new_field.ty, mode, new_field.span, &what)?;
            if self.older.default_of(old_field) != self.newer.default_of(new_field) {
                return Err(DefinitionError::new(
                    new_field.span,
                    format!("changes the default of `{}`", new_field.name),
                ));
            }

            let since = self.old_additions.field(old_field);

            self.additions.set_field(new_field.id, since);
        }

        for new_extra in new.iter().skip(old.len()) {
            if mode == Mode::Exact {
                return Err(Self::reject_addition(new_extra.span, &new_extra.name));
            }

            if !(new_extra.optional || new_extra.default.is_some()) || new_extra.flatten {
                return Err(DefinitionError::new(
                    new_extra.span,
                    format!(
                        "adds `{}`, which is neither optional nor has a default; a minor version adds fields older peers can omit",
                        new_extra.name
                    ),
                ));
            }

            self.additions.set_field(new_extra.id, self.minor);
        }

        Ok(())
    }

    pub(super) fn variants(
        &mut self,
        old: &[Variant],
        new: &[Variant],
        mode: Mode,
        span: Span,
    ) -> Result<(), DefinitionError> {
        if let Some(old_extra) = old.get(new.len()) {
            return Err(DefinitionError::new(
                span,
                format!("removes the variant `{}`", old_extra.name),
            ));
        }

        for (old_variant, new_variant) in old.iter().zip(new) {
            let span = new_variant.span;

            match (&old_variant.payload, &new_variant.payload) {
                (Payload::Unit, Payload::Unit) => {}
                (Payload::Value(old_value), Payload::Value(new_value)) => {
                    let what = format!("`{}`", new_variant.name);

                    self.ty(old_value, new_value, mode, span, &what)?;
                }
                (Payload::Fields(old_fields), Payload::Fields(new_fields)) => {
                    self.fields(old_fields, new_fields, mode, span)?;
                }
                _ => {
                    return Err(DefinitionError::new(
                        span,
                        format!("changes what `{}` carries", new_variant.name),
                    ));
                }
            }

            let since = self.old_additions.variant(old_variant);

            self.additions.set_variant(new_variant.id, since);
        }

        for new_extra in new.iter().skip(old.len()) {
            if mode == Mode::Exact {
                return Err(Self::reject_addition(new_extra.span, &new_extra.name));
            }

            self.additions.set_variant(new_extra.id, self.minor);
        }

        Ok(())
    }

    pub(super) fn members(
        &mut self,
        old: &[Member],
        new: &[Member],
        mode: Mode,
        span: Span,
    ) -> Result<(), DefinitionError> {
        if let Some(old_extra) = old.get(new.len()) {
            return Err(DefinitionError::new(
                span,
                format!("removes the member `{}`", old_extra.name),
            ));
        }

        for (index, (old_member, new_member)) in (0..).zip(old.iter().zip(new)) {
            if old_member.value.unwrap_or(index) != new_member.value.unwrap_or(index) {
                return Err(DefinitionError::new(
                    new_member.span,
                    format!("changes the value of `{}`", new_member.name),
                ));
            }

            let since = self.old_additions.member(old_member);

            self.additions.set_member(new_member.id, since);
        }

        for new_extra in new.iter().skip(old.len()) {
            if mode == Mode::Exact {
                return Err(Self::reject_addition(new_extra.span, &new_extra.name));
            }

            self.additions.set_member(new_extra.id, self.minor);
        }

        Ok(())
    }
}
