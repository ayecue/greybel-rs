use serde::ser::{Serialize, SerializeStruct, Serializer};

use super::Definition;
use crate::description::Description;

#[derive(Debug, Clone, Copy)]
pub struct LocalizedDefinition<'a> {
    origin: &'a str,
    name: &'a str,
    definition: &'a Definition,
    description: Option<&'a Description>,
}

impl<'a> LocalizedDefinition<'a> {
    pub(crate) fn new(
        origin: &'a str,
        name: &'a str,
        definition: &'a Definition,
        description: Option<&'a Description>,
    ) -> Self {
        Self {
            origin,
            name,
            definition,
            description,
        }
    }

    #[must_use]
    pub fn origin(&self) -> &'a str {
        self.origin
    }

    #[must_use]
    pub fn name(&self) -> &'a str {
        self.name
    }

    #[must_use]
    pub fn definition(&self) -> &'a Definition {
        self.definition
    }

    #[must_use]
    pub fn description(&self) -> Option<&'a str> {
        match self.description {
            Some(description) => Some(description.text()),
            None => self.definition.description(),
        }
    }

    #[must_use]
    pub fn example(&self) -> Option<&'a [String]> {
        self.description
            .and_then(Description::example)
            .or_else(|| self.definition.example())
    }
}

impl Serialize for LocalizedDefinition<'_> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let function = self.definition.as_function();
        let fields = if function.is_some() { 6 } else { 4 };
        let mut object = serializer.serialize_struct("Definition", fields)?;

        object.serialize_field("type", self.definition.type_meta())?;
        object.serialize_field("isProtected", &self.definition.is_protected())?;
        object.serialize_field("description", &self.description())?;
        object.serialize_field("example", &self.example())?;
        if let Some(function) = function {
            object.serialize_field("arguments", function.arguments())?;
            object.serialize_field("returns", function.returns())?;
        }

        object.end()
    }
}
