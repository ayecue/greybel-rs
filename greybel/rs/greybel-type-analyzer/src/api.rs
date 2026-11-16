use std::sync::OnceLock;

use greybel_core::Dialect;
use meta_utils::{BaseType, Container, DEFAULT_LANGUAGE, LocalizedDefinition, TypeLookup};
use rustc_hash::FxHashSet;

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

    pub(crate) fn singleton_map_functions(self) -> &'static [&'static str] {
        match self.dialect {
            Dialect::MiniScript => &[],
            Dialect::Greybel | Dialect::GreyScript => &["get_custom_object"],
        }
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
        if type_name == BaseType::Any.as_str() {
            return self
                .container
                .signature(type_name)?
                .definition(name, DEFAULT_LANGUAGE);
        }

        self.container
            .definition(&[type_name], name, DEFAULT_LANGUAGE)
    }

    pub(crate) fn is_general_function(self, name: &str) -> bool {
        static MINISCRIPT: OnceLock<FxHashSet<&'static str>> = OnceLock::new();
        static GREYSCRIPT: OnceLock<FxHashSet<&'static str>> = OnceLock::new();

        let names = match self.dialect {
            Dialect::MiniScript => &MINISCRIPT,
            Dialect::Greybel | Dialect::GreyScript => &GREYSCRIPT,
        };

        names
            .get_or_init(|| {
                self.members(BaseType::General.as_str())
                    .filter(|member| {
                        member.definition().as_function().is_some()
                            && !member.definition().is_detached()
                    })
                    .map(|member| member.name())
                    .collect()
            })
            .contains(name)
    }

    pub(crate) fn adds_to_container(self, name: &str) -> bool {
        static MINISCRIPT: OnceLock<FxHashSet<&'static str>> = OnceLock::new();
        static GREYSCRIPT: OnceLock<FxHashSet<&'static str>> = OnceLock::new();

        self.has_mutation(
            &MINISCRIPT,
            &GREYSCRIPT,
            &[BaseType::List, BaseType::Map],
            name,
        )
    }

    pub(crate) fn adds_to_argument(self, name: &str) -> bool {
        static MINISCRIPT: OnceLock<FxHashSet<&'static str>> = OnceLock::new();
        static GREYSCRIPT: OnceLock<FxHashSet<&'static str>> = OnceLock::new();

        self.has_mutation(&MINISCRIPT, &GREYSCRIPT, &[BaseType::General], name)
    }

    fn has_mutation(
        self,
        miniscript: &'static OnceLock<FxHashSet<&'static str>>,
        greyscript: &'static OnceLock<FxHashSet<&'static str>>,
        bases: &[BaseType],
        name: &str,
    ) -> bool {
        let names = match self.dialect {
            Dialect::MiniScript => miniscript,
            Dialect::Greybel | Dialect::GreyScript => greyscript,
        };

        names
            .get_or_init(|| {
                bases
                    .iter()
                    .flat_map(|base| self.members(base.as_str()))
                    .filter(|member| {
                        member
                            .definition()
                            .as_function()
                            .is_some_and(|function| function.mutation().is_some())
                    })
                    .map(|member| member.name())
                    .collect()
            })
            .contains(name)
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
