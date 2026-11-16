mod edit;

use std::sync::Arc;

pub use edit::{TextEdit, WorkspaceEdit};
use greybel_core::syntax::IdentifierKind;
use greybel_core::{Ast, Lexer, NodeKind, Position, Range, Span};
use greybel_documents::{SourceFile, Uri};
use greybel_symbols::contextual_keywords::AS;

use crate::analyzer::Analyzer;
use crate::database::Annotate;
use crate::ide::{Context, Location};

#[derive(Clone, Copy)]
enum Shorthand {
    MapLiteral,
    DestructuringTarget,
}

impl<'w> Analyzer<'w> {
    #[must_use]
    pub fn prepare_rename(&self, uri: &Uri, position: Position) -> Option<Range> {
        let (file, ast, offset) = self.locate(uri, position)?;

        if let Some((name, _)) = self.documented_parameter_at(file, ast, offset) {
            return Some(Self::range(ast, name));
        }

        self.analyses_of(file)
            .into_iter()
            .find_map(|(analysis, document)| {
                let (node, _) = Context::new(ast, analysis, document, offset).referenced_slots()?;

                Some(Self::range(ast, ast.span(node)))
            })
    }

    #[must_use]
    pub fn rename(&self, uri: &Uri, position: Position, name: &str) -> Option<WorkspaceEdit> {
        self.prepare_rename(uri, position)?;
        if !self.is_identifier(name) {
            return None;
        }

        let (file, ast, offset) = self.locate(uri, position)?;
        let position = match self.documented_parameter_at(file, ast, offset) {
            Some((_, parameter)) => Self::range(ast, Span::empty(parameter)).start,
            None => position,
        };
        let renames_variable = self.renames_variable(file, ast, offset);
        let mut locations = self.references(uri, position);

        locations.extend(self.param_tag_locations(uri, position));

        let mut edit = WorkspaceEdit::default();

        for location in Self::sort_locations(locations) {
            let text = match self.shared_name_at(&location) {
                Some((Shorthand::MapLiteral, old)) if renames_variable => {
                    format!("\"{old}\": {name}")
                }
                Some((Shorthand::MapLiteral, old)) => format!("\"{name}\": {old}"),
                Some((Shorthand::DestructuringTarget, old)) if renames_variable => {
                    format!("{old} {AS} {name}")
                }
                Some((Shorthand::DestructuringTarget, old)) => format!("{name} {AS} {old}"),
                None => name.to_owned(),
            };

            edit.add(location, text);
        }

        Some(edit)
    }

    fn renames_variable(self, file: SourceFile, ast: &Ast, offset: u32) -> bool {
        self.analyses_of(file)
            .into_iter()
            .find_map(|(analysis, document)| {
                let (node, _) = Context::new(ast, analysis, document, offset).referenced_slots()?;

                Some(match ast.kind(node) {
                    NodeKind::Identifier(identifier) => identifier.kind != IdentifierKind::Property,
                    _ => false,
                })
            })
            .unwrap_or(false)
    }

    fn shared_name_at(self, location: &Location) -> Option<(Shorthand, &'w str)> {
        let file = self.workspace().file(&location.uri)?;
        let ast = file.parse(self.workspace().db()).ast()?;

        ast.nodes().iter().find_map(|(_, node)| {
            let (key, variable, shorthand) = match &node.kind {
                NodeKind::MapEntry(entry) if entry.shorthand => {
                    (entry.key, entry.value, Shorthand::MapLiteral)
                }
                NodeKind::Assignment(assignment) => {
                    let NodeKind::Unary(unary) = ast.kind(assignment.init) else {
                        return None;
                    };
                    let key = match ast.kind(unary.argument) {
                        NodeKind::Member(member) => member.identifier,
                        NodeKind::Index(index) => index.index,
                        _ => return None,
                    };

                    (key, assignment.variable, Shorthand::DestructuringTarget)
                }
                _ => return None,
            };
            let span = ast.span(variable);

            (ast.span(key) == span && !span.is_empty() && Self::range(ast, span) == location.range)
                .then(|| (shorthand, ast.slice(span)))
        })
    }

    fn documented_parameter_at(
        self,
        file: SourceFile,
        ast: &Ast,
        offset: u32,
    ) -> Option<(Span, u32)> {
        let comments = file.comments(self.workspace().db());
        let (block, name, span) = comments.blocks.iter().find_map(|block| {
            block
                .param_names()
                .find(|(_, span)| span.start <= offset && offset <= span.end)
                .map(|(param, span)| (block, param.name.as_str(), span))
        })?;

        self.analyses_of(file)
            .into_iter()
            .find_map(|(analysis, document)| {
                let model = analysis.model();

                model.functions().find_map(|function| {
                    let data = model.function(function);
                    let documents_function = data.site.document == document
                        && data
                            .doc
                            .as_ref()
                            .is_some_and(|doc| Arc::ptr_eq(doc, &block.doc));
                    let parameter = data
                        .parameters
                        .iter()
                        .find(|parameter| documents_function && parameter.name == name)?;

                    Some((span, ast.span(parameter.node).start))
                })
            })
    }

    fn param_tag_locations(self, uri: &Uri, position: Position) -> Vec<Location> {
        let Some((file, ast, offset)) = self.locate(uri, position) else {
            return Vec::new();
        };
        let db = self.workspace().db();
        let mut locations = Vec::new();

        for (analysis, document) in self.analyses_of(file) {
            let Some((_, slots)) = Context::new(ast, analysis, document, offset).referenced_slots()
            else {
                continue;
            };
            let model = analysis.model();

            for function in model.functions() {
                let data = model.function(function);
                let parameter = data
                    .parameters
                    .iter()
                    .find(|parameter| slots.contains(&parameter.slot));
                let (Some(parameter), Some(doc)) = (parameter, &data.doc) else {
                    continue;
                };
                let Some(&doc_file) = analysis.documents().get(data.site.document as usize) else {
                    continue;
                };
                let Some(doc_ast) = doc_file.parse(db).ast() else {
                    continue;
                };
                let comments = doc_file.comments(db);
                let Some(block) = comments
                    .blocks
                    .iter()
                    .find(|block| Arc::ptr_eq(&block.doc, doc))
                else {
                    continue;
                };

                locations.extend(
                    block
                        .param_names()
                        .filter(|(param, _)| param.name == parameter.name)
                        .map(|(_, span)| Location {
                            uri: doc_file.uri(db).clone(),
                            range: Self::range(doc_ast, span),
                        }),
                );
            }
        }

        locations
    }

    fn is_identifier(self, name: &str) -> bool {
        let options = &self.workspace().options().parser;
        let lexer = Lexer::with_grammar(name, options.grammar());
        let mut tokens = lexer.tokens();
        let first = tokens.next().and_then(|token| token.ok());
        let identifier = first
            .as_ref()
            .and_then(|token| token.as_identifier())
            .is_some_and(|identifier| identifier.name(name) == name);

        identifier && tokens.next().is_none()
    }
}
