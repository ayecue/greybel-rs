use greybel_core::{Ast, Span};

use super::Context;
use crate::model::{DocumentIndex, MapId, MapOrigin, Site};
use crate::types::Type;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TypeDefinition {
    /// The comment line of a `@type` or `@vtype` tag.
    Tag(DocumentIndex, Span),
    /// A map literal or `new` expression.
    Node(Site),
}

impl TypeDefinition {
    pub(crate) const fn document(self) -> DocumentIndex {
        match self {
            Self::Tag(document, _) => document,
            Self::Node(site) => site.document,
        }
    }

    /// The span in `ast`, the syntax tree of [`Self::document`].
    pub(crate) fn span(self, ast: &Ast) -> Span {
        match self {
            Self::Tag(_, span) => span,
            Self::Node(site) => ast.span(site.node),
        }
    }
}

impl Context<'_> {
    /// Where the types of the value at the offset are declared: the `@type` tag of a custom type, or the map literal
    /// or `new` expression that created a map.
    pub(crate) fn type_definitions(&self) -> Vec<TypeDefinition> {
        let Some(ty) = self
            .ast
            .path_at(self.offset)
            .into_iter()
            .rev()
            .find_map(|node| self.analysis.node_type(self.document, node))
        else {
            return Vec::new();
        };
        let types = self.analysis.types();
        let mut definitions = Vec::new();

        for variant in types.variants(ty) {
            if let Type::Object(map) = *types.get(variant)
                && let Some(definition) = self.map_definition(map)
                && !definitions.contains(&definition)
            {
                definitions.push(definition);
            }
        }

        definitions
    }

    fn map_definition(&self, map: MapId) -> Option<TypeDefinition> {
        let model = self.analysis.model();
        let mut map = map;

        for _ in 0..16 {
            if let Some(custom) = model.custom_type_of(map) {
                return Some(TypeDefinition::Tag(custom.document, custom.span));
            }

            map = match model.map(map).origin {
                MapOrigin::Instance(class) => class,
                MapOrigin::GenericInstance(id) => model.generic_instance(id).generic,
                MapOrigin::New(site) => match self.class_of(map) {
                    Some(class) => class,
                    None => return Some(TypeDefinition::Node(site)),
                },
                MapOrigin::Literal(site) => return Some(TypeDefinition::Node(site)),
                _ => return None,
            };
        }

        None
    }

    /// The map in the `__isa` entry of `map`, the class a `new` expression made it from.
    fn class_of(&self, map: MapId) -> Option<MapId> {
        let model = self.analysis.model();
        let types = self.analysis.types();
        let isa = model.slot_type(model.map(map).isa);

        types
            .variants(isa)
            .find_map(|variant| match *types.get(variant) {
                Type::Object(class) => Some(class),
                _ => None,
            })
    }
}
