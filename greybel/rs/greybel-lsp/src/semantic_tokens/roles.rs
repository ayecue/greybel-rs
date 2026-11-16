use std::collections::HashMap;

use greybel_core::syntax::greybel::TypeBody;
use greybel_core::{Ast, IdentifierKind, Node, NodeId, NodeKind, Span, Visitor};
use greybel_symbols::contextual_keywords;

use super::token_type::TokenType;

pub(super) struct IdentifierRoles<'a> {
    type_spans: Vec<Span>,
    ast: &'a Ast,
    types: HashMap<u32, TokenType>,
}

impl<'a> IdentifierRoles<'a> {
    pub(super) fn of(ast: &'a Ast) -> Self {
        let mut type_spans = ast.type_spans();

        type_spans.sort_unstable_by_key(|span| span.start);

        let mut roles = Self {
            type_spans,
            ast,
            types: HashMap::new(),
        };

        ast.walk(ast.root(), &mut roles);

        roles
    }

    pub(super) fn of_token(&self, span: Span) -> TokenType {
        if let Some(&token_type) = self.types.get(&span.start) {
            return token_type;
        }

        let text = &self.ast.source()[span.start as usize..span.end as usize];

        if contextual_keywords::STATEMENT_KEYWORDS.contains(&text) {
            return TokenType::Keyword;
        }

        let index = self.type_spans.partition_point(|ty| ty.start <= span.start);
        let in_type = self.type_spans[..index].iter().any(|ty| span.end <= ty.end);

        if in_type {
            TokenType::Type
        } else {
            TokenType::Variable
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
                let token_type = match identifier.kind {
                    IdentifierKind::Argument => TokenType::Parameter,
                    IdentifierKind::Property => TokenType::Property,
                    IdentifierKind::Variable | IdentifierKind::ForInVariable => TokenType::Variable,
                };

                self.types.entry(node.span.start).or_insert(token_type);
            }
            NodeKind::Call(call) => {
                if let Some(callee) = self.called_identifier(call.base) {
                    self.types
                        .insert(self.ast.span(callee).start, TokenType::Function);
                }
            }
            NodeKind::CallStatement(statement) => {
                if let Some(callee) = self.called_identifier(statement.expression) {
                    self.types
                        .insert(self.ast.span(callee).start, TokenType::Function);
                }
            }
            NodeKind::TypeStatement(statement) => {
                if let TypeBody::Members { members, .. } = &statement.body {
                    for member in members {
                        let token_type = match member.parameters {
                            Some(_) => TokenType::Function,
                            None => TokenType::Property,
                        };

                        self.types.insert(member.name.start, token_type);

                        for &(parameter, _) in member.parameters.iter().flatten() {
                            self.types.insert(parameter.start, TokenType::Parameter);
                        }
                    }
                }
            }
            _ => {}
        }

        true
    }
}
