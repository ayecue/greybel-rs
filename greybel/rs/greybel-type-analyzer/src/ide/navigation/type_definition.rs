use greybel_core::{Ast, ColumnSettings, Position, Span};
use greybel_documents::Uri;

use super::Location;
use crate::analyzer::Analyzer;
use crate::ide::Context;
use crate::model::{DocumentIndex, MapId, MapOrigin, Site};
use crate::types::Type;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TypeDefinition {
    Tag(DocumentIndex, Span),
    Node(Site),
}

impl TypeDefinition {
    const fn document(self) -> DocumentIndex {
        match self {
            Self::Tag(document, _) => document,
            Self::Node(site) => site.document,
        }
    }

    fn span(self, ast: &Ast) -> Span {
        match self {
            Self::Tag(_, span) => span,
            Self::Node(site) => ast.span(site.node),
        }
    }
}

impl Context<'_> {
    fn type_definitions(&self) -> Vec<TypeDefinition> {
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

impl Analyzer<'_> {
    #[must_use]
    pub fn type_definition(&self, uri: &Uri, position: Position) -> Vec<Location> {
        let Some((file, ast, offset)) = self.locate(uri, position) else {
            return Vec::new();
        };
        let db = self.workspace().db();
        let mut locations = Vec::new();

        for (analysis, document) in self.analyses_of(file) {
            for definition in Context::new(ast, analysis, document, offset).type_definitions() {
                let Some(&target) = analysis.documents().get(definition.document() as usize) else {
                    continue;
                };
                let Some(target_ast) = target.parse(db).ast() else {
                    continue;
                };
                let span = definition.span(target_ast);

                locations.push(Location {
                    uri: target.uri(db).clone(),
                    range: target_ast.lines().range(span, ColumnSettings::default()),
                });
            }
        }

        Self::sort_locations(locations)
    }
}
