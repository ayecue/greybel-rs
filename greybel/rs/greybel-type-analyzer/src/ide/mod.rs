mod code_action;
mod completion;
mod hover;
mod imports;
mod navigation;
mod rename;
mod signature;
mod symbols;
mod text;

pub use code_action::CodeAction;
pub use completion::{CompletionItem, CompletionItemKind};
use greybel_core::{Ast, NodeId, NodeKind, Span};
pub use hover::Hover;
pub(crate) use imports::{ImportTarget, RelativePath};
pub use navigation::Location;
pub use rename::{TextEdit, WorkspaceEdit};
pub use signature::{ParameterInformation, SignatureHelp};
pub(crate) use symbols::SymbolQuery;
pub use symbols::{DocumentSymbol, SymbolKind, WorkspaceSymbol};
use text::SourceText;

use crate::analysis::Analysis;
use crate::model::{DocumentIndex, ScopeId, Site, SlotId};
use crate::query::Query;
use crate::types::TypeId;

pub(crate) struct Context<'a> {
    ast: &'a Ast,
    analysis: &'a Analysis,
    document: DocumentIndex,
    offset: u32,
}

impl<'a> Context<'a> {
    pub(crate) fn new(
        ast: &'a Ast,
        analysis: &'a Analysis,
        document: DocumentIndex,
        offset: u32,
    ) -> Self {
        Self {
            ast,
            analysis,
            document,
            offset,
        }
    }

    fn source_text(&self) -> SourceText<'a> {
        SourceText::new(self.ast.source())
    }

    fn scope(&self) -> ScopeId {
        let model = self.analysis.model();
        let mut best: Option<(Span, ScopeId)> = None;

        for function in model.functions() {
            let site = model.function(function).site;

            if site.document != self.document {
                continue;
            }

            let span = self.ast.span(site.node);
            let inside = span.start <= self.offset && self.offset <= span.end;
            let narrower = best.is_none_or(|(candidate, _)| {
                span.end - span.start < candidate.end - candidate.start
            });

            if inside && narrower {
                best = Some((span, ScopeId::Function(function)));
            }
        }

        best.map_or(ScopeId::Global, |(_, scope)| scope)
    }

    fn is_reference(&self, node: NodeId) -> bool {
        matches!(
            self.ast.kind(node),
            NodeKind::Identifier(_) | NodeKind::Member(_) | NodeKind::Index(_)
        )
    }

    fn typed_node_at_end(
        &self,
        end: u32,
        accept: impl Fn(NodeId) -> bool,
    ) -> Option<(NodeId, TypeId)> {
        self.ast
            .path_at(end.saturating_sub(1))
            .into_iter()
            .rev()
            .find_map(|node| {
                let ends_here = self.ast.span(node).end == end && accept(node);

                ends_here.then(|| Some((node, self.analysis.node_type(self.document, node)?)))?
            })
    }

    fn resolve_variable_type(
        &self,
        query: &mut Query<'_>,
        scope: ScopeId,
        name: &str,
    ) -> Option<TypeId> {
        let model = self.analysis.model();

        match model.resolve_variable(scope, name) {
            Some(slot) => Some(model.slot_type(slot)),
            None => query.general_function_type(name),
        }
    }

    fn chain_type(&self, query: &mut Query<'_>, end: u32, invoke: bool) -> Option<TypeId> {
        let names = self.source_text().member_chain_before(end as usize)?;
        let (first, remainder) = names.split_first()?;
        let mut ty = self.resolve_variable_type(query, self.scope(), first)?;

        for name in remainder {
            let value = query.invocation_type(ty);

            ty = query.member_type(value, name);
        }

        Some(if invoke {
            query.invocation_type(ty)
        } else {
            ty
        })
    }

    pub(crate) fn referenced_slots(&self) -> Option<(NodeId, &'a [SlotId])> {
        let references = self.analysis.references();

        self.ast
            .path_at(self.offset)
            .into_iter()
            .rev()
            .find_map(|node| {
                let slots = references.slots(Site {
                    document: self.document,
                    node,
                });

                (!slots.is_empty()).then_some((node, slots))
            })
    }
}
