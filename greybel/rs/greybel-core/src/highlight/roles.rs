use std::collections::HashMap;

use greybel_symbols::contextual_keywords;

use crate::syntax::greybel::TypeBody;
use crate::{Ast, IdentifierKind, Node, NodeId, NodeKind, Span, Visitor};

use super::HighlightKind;

pub(super) struct IdentifierRoles<'a> {
    type_spans: Vec<Span>,
    ast: &'a Ast,
    kinds: HashMap<u32, HighlightKind>,
}

impl<'a> IdentifierRoles<'a> {
    pub(super) fn of(ast: &'a Ast) -> Self {
        let mut type_spans = ast.type_spans();

        type_spans.sort_unstable_by_key(|span| span.start);

        let mut roles = Self {
            type_spans,
            ast,
            kinds: HashMap::new(),
        };

        ast.walk(ast.root(), &mut roles);

        roles
    }

    pub(super) fn of_token(&self, span: Span) -> HighlightKind {
        if let Some(&kind) = self.kinds.get(&span.start) {
            return kind;
        }

        let text = &self.ast.source()[span.start as usize..span.end as usize];

        if contextual_keywords::STATEMENT_KEYWORDS.contains(&text) {
            return HighlightKind::Keyword;
        }

        let index = self.type_spans.partition_point(|ty| ty.start <= span.start);
        let in_type = self.type_spans[..index].iter().any(|ty| span.end <= ty.end);

        if in_type {
            HighlightKind::Type
        } else {
            HighlightKind::Variable
        }
    }

    fn called_identifier(&self, callee: NodeId) -> Option<NodeId> {
        match self.ast.kind(callee) {
            NodeKind::Identifier(_) => Some(callee),
            NodeKind::Member(member) => Some(member.identifier),
            _ => None,
        }
    }
}

impl Visitor for IdentifierRoles<'_> {
    fn enter(&mut self, _id: NodeId, node: &Node) -> bool {
        match &node.kind {
            NodeKind::Identifier(identifier) => {
                let kind = match identifier.kind {
                    IdentifierKind::Argument => HighlightKind::Parameter,
                    IdentifierKind::Property => HighlightKind::Property,
                    IdentifierKind::Variable | IdentifierKind::ForInVariable => {
                        HighlightKind::Variable
                    }
                };

                self.kinds.entry(node.span.start).or_insert(kind);
            }
            NodeKind::Call(call) => {
                if let Some(callee) = self.called_identifier(call.base) {
                    self.kinds
                        .insert(self.ast.span(callee).start, HighlightKind::Function);
                }
            }
            NodeKind::CallStatement(statement) => {
                if let Some(callee) = self.called_identifier(statement.expression) {
                    self.kinds
                        .insert(self.ast.span(callee).start, HighlightKind::Function);
                }
            }
            NodeKind::TypeStatement(statement) => {
                if let TypeBody::Members { members, .. } = &statement.body {
                    for member in members {
                        let kind = match member.parameters {
                            Some(_) => HighlightKind::Function,
                            None => HighlightKind::Property,
                        };

                        self.kinds.insert(member.name.start, kind);

                        for &(parameter, _) in member.parameters.iter().flatten() {
                            self.kinds.insert(parameter.start, HighlightKind::Parameter);
                        }
                    }
                }
            }
            _ => {}
        }

        true
    }
}
