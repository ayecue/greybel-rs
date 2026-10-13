use greybel_core::ast::Scope;
use greybel_core::{Ast, NodeFieldContainer, NodeKind, ScopeData};

pub fn function_scopes(ast: &Ast) -> Vec<ScopeData> {
    ast.scopes
        .iter()
        .filter_map(|&scope| ast.kind(scope).as_scope().map(Scope::scope).copied())
        .collect()
}

pub fn names(ast: &Ast, ids: &[greybel_core::NodeId]) -> Vec<String> {
    ids.iter()
        .filter_map(|&id| match ast.kind(id) {
            NodeKind::Assignment(assignment) => ast.text(assignment.variable).map(str::to_owned),
            NodeKind::Identifier(identifier) => Some(ast.resolve(identifier.name).to_owned()),
            _ => None,
        })
        .collect()
}
