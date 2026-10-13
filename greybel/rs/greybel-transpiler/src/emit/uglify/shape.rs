use std::borrow::Cow;

use greybel_core::syntax::{Call, Literal};
use greybel_core::{LiteralValue, NodeId, NodeKind};
use greybel_symbols::names::SCOPES;

use crate::emit::EmitState;
use crate::emit::nodes::unwrap_from_parens;

const HAS_INDEX: &str = "hasIndex";

pub(super) fn is_scope_name(s: &EmitState<'_>, id: NodeId) -> bool {
    matches!(
        s.kind(unwrap_from_parens(s, id)),
        NodeKind::Identifier(name) if SCOPES.contains(&s.resolve(name.name))
    )
}

pub(super) fn has_index_key_argument(s: &EmitState<'_>, node: &Call) -> Option<NodeId> {
    let NodeKind::Member(member) = s.kind(unwrap_from_parens(s, node.base)) else {
        return None;
    };
    let is_has_index = matches!(
        s.kind(member.identifier),
        NodeKind::Identifier(name) if s.resolve(name.name) == HAS_INDEX
    );

    (is_has_index && is_scope_name(s, member.base))
        .then(|| s.ast().list(node.arguments).first().copied())
        .flatten()
}

pub(super) fn string_value<'a>(s: &EmitState<'a>, id: NodeId) -> Option<Cow<'a, str>> {
    match s.kind(id) {
        NodeKind::Literal(Literal {
            value: LiteralValue::String(value),
            ..
        }) => Some(s.read(*value)),
        _ => None,
    }
}
