use std::borrow::Cow;

use greybel_core::syntax::{Call, Literal};
use greybel_core::{LiteralValue, NodeId, NodeKind};
use greybel_symbols::names::SCOPES;

use crate::emit::EmitState;

const HAS_INDEX: &str = "hasIndex";

impl<'a> EmitState<'a> {
    pub(super) fn is_scope_name(&self, id: NodeId) -> bool {
        matches!(
            self.kind(self.unwrap_from_parens(id)),
            NodeKind::Identifier(name) if SCOPES.contains(&self.resolve(name.name))
        )
    }

    pub(super) fn has_index_key_argument(&self, node: &Call) -> Option<NodeId> {
        let NodeKind::Member(member) = self.kind(self.unwrap_from_parens(node.base)) else {
            return None;
        };
        let is_has_index = matches!(
            self.kind(member.identifier),
            NodeKind::Identifier(name) if self.resolve(name.name) == HAS_INDEX
        );

        (is_has_index && self.is_scope_name(member.base))
            .then(|| self.ast().list(node.arguments).first().copied())
            .flatten()
    }

    pub(super) fn string_value(&self, id: NodeId) -> Option<Cow<'a, str>> {
        match self.kind(id) {
            NodeKind::Literal(Literal {
                value: LiteralValue::String(value),
                ..
            }) => Some(self.read(*value)),
            _ => None,
        }
    }
}
