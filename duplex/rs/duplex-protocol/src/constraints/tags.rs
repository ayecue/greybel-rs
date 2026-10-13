use std::collections::HashSet;

use super::ConstraintTable;
use crate::error::DefinitionError;
use crate::model::{Field, Payload, TagList, TagPlace, TypeKind};

impl ConstraintTable<'_> {
    pub(super) fn verify_tags(&self) -> Result<(), DefinitionError> {
        let mut names = HashSet::new();

        for declaration in &self.definition.tags {
            let name = &declaration.name;

            if !Self::is_snake_case(name) {
                return Err(DefinitionError::new(
                    declaration.span,
                    format!("`{name}` is a tag's name in snake_case"),
                ));
            }

            if !names.insert(name.as_str()) {
                return Err(DefinitionError::new(
                    declaration.span,
                    format!("the tag `{name}` is declared twice"),
                ));
            }

            if let Some(ty) = &declaration.ty
                && !Self::is_taking_literals(self.definition, ty)
            {
                return Err(DefinitionError::new(
                    declaration.span,
                    format!(
                        "the tag `{name}` is of {}; a tag's value is a boolean, character, text, number or member of an enumeration",
                        ty.keyword()
                    ),
                ));
            }
        }

        if let Some(protocol) = &self.definition.protocol {
            self.verify_tags_of(&protocol.tags, TagPlace::Protocol)?;
            for message in &protocol.messages {
                self.verify_tags_of(&message.tags, TagPlace::Message)?;
                self.verify_field_tags(&message.fields)?;
            }
        }

        for declaration in &self.definition.types {
            match &declaration.kind {
                TypeKind::Struct(fields) => {
                    self.verify_tags_of(&declaration.tags, TagPlace::Struct)?;
                    self.verify_field_tags(fields)?;
                }
                TypeKind::Union(variants) => {
                    self.verify_tags_of(&declaration.tags, TagPlace::Union)?;
                    for variant in variants {
                        self.verify_tags_of(&variant.tags, TagPlace::Variant)?;
                        if let Payload::Fields(fields) = &variant.payload {
                            self.verify_field_tags(fields)?;
                        }
                    }
                }
                TypeKind::Enumeration(members) => {
                    self.verify_tags_of(&declaration.tags, TagPlace::Enumeration)?;
                    for member in members {
                        self.verify_tags_of(&member.tags, TagPlace::Member)?;
                    }
                }
            }
        }

        Ok(())
    }

    fn verify_field_tags(&self, fields: &[Field]) -> Result<(), DefinitionError> {
        for field in fields {
            self.verify_tags_of(&field.tags, TagPlace::Field)?;
        }

        Ok(())
    }

    /// Validates that each tag is declared, permitted at `place`, has its declared value type, and occurs only once
    /// unless declared `repeated`.
    fn verify_tags_of(&self, tags: &TagList, place: TagPlace) -> Result<(), DefinitionError> {
        let mut put = HashSet::new();

        for tag in tags.all() {
            let name = &tag.name;
            let Some(declaration) = self.definition.tag_declaration(name) else {
                return Err(DefinitionError::new(
                    tag.span,
                    format!(
                        "the tag `{name}` isn't declared: declare it as `tag {name}: Type on {};`",
                        place.keyword()
                    ),
                ));
            };

            if !declaration.places.contains(&place) {
                let places: Vec<String> = declaration
                    .places
                    .iter()
                    .map(|place| format!("`{}`", place.keyword()))
                    .collect();

                return Err(DefinitionError::new(
                    tag.span,
                    format!(
                        "the tag `{name}` is put on {}, not on a `{}`",
                        places.join(", "),
                        place.keyword()
                    ),
                ));
            }

            if !declaration.repeatable && !put.insert(name.as_str()) {
                return Err(DefinitionError::new(
                    tag.span,
                    format!(
                        "the tag `{name}` is put on an item once, unless it is declared `repeated`"
                    ),
                ));
            }

            match (&declaration.ty, &tag.value) {
                (None, None) => {}
                (None, Some(_)) => {
                    return Err(DefinitionError::new(
                        tag.span,
                        format!("the tag `{name}` is a flag: it takes no value"),
                    ));
                }
                (Some(ty), None) => {
                    return Err(DefinitionError::new(
                        tag.span,
                        format!("the tag `{name}` takes a value of {}", ty.keyword()),
                    ));
                }
                (Some(ty), Some(value)) => {
                    self.definition.value_of(value, ty).map_err(|error| {
                        DefinitionError::new(tag.span, format!("the tag `{name}`'s value {error}"))
                    })?;
                }
            }
        }

        Ok(())
    }
}
