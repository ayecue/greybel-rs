use greybel_core::Dialect;
use meta_utils::{Container, DEFAULT_LANGUAGE, LocalizedDefinition, TypeLookup};

use crate::types::ApiFunction;

#[derive(Debug, Clone, Copy)]
pub struct Api {
    dialect: Dialect,
    container: &'static Container,
}

impl PartialEq for Api {
    fn eq(&self, other: &Self) -> bool {
        self.dialect == other.dialect
    }
}

impl Eq for Api {}

impl Api {
    #[must_use]
    pub fn new(dialect: Dialect) -> Self {
        let container = match dialect {
            Dialect::MiniScript => miniscript_meta::container(),
            Dialect::Greybel | Dialect::GreyScript => greyscript_meta::container(),
        };

        Self { dialect, container }
    }

    #[must_use]
    pub fn dialect(self) -> Dialect {
        self.dialect
    }

    #[must_use]
    pub fn container(self) -> &'static Container {
        self.container
    }

    pub(crate) fn member(
        self,
        type_name: &str,
        name: &str,
    ) -> Option<LocalizedDefinition<'static>> {
        self.container
            .definition(&[type_name], name, DEFAULT_LANGUAGE)
    }

    pub(crate) fn members(
        self,
        type_name: &str,
    ) -> impl Iterator<Item = LocalizedDefinition<'static>> + use<> {
        self.container.members(type_name, DEFAULT_LANGUAGE)
    }

    pub(crate) fn definition(self, function: ApiFunction) -> Option<LocalizedDefinition<'static>> {
        self.container
            .signature(function.origin)?
            .definition(function.name, DEFAULT_LANGUAGE)
    }

    pub(crate) fn class_name(self, name: &str) -> Option<&'static str> {
        match self.container.lookup_type(name) {
            TypeLookup::Class(signature) => Some(signature.type_name()),
            _ => None,
        }
    }

    pub(crate) fn lookup_type(self, name: &str) -> TypeLookup<'static> {
        self.container.lookup_type(name)
    }

    pub(crate) fn type_parameters(self, type_name: &str) -> &'static [String] {
        self.container
            .signature(type_name)
            .map_or(&[], |signature| signature.type_parameters())
    }

    pub(crate) fn inherits(self, class: &str, ancestor: &str) -> bool {
        let mut class = Some(class);

        for _ in 0..32 {
            match class {
                Some(name) if name == ancestor => return true,
                Some(name) => {
                    class = self
                        .container
                        .signature(name)
                        .and_then(|signature| signature.extends());
                }
                None => return false,
            }
        }

        false
    }
}
