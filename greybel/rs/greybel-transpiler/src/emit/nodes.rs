use greybel_core::ast::NodeList;
use greybel_core::syntax::{ArrowBody, Assignment, Function};
use greybel_core::{NodeId, NodeKind};

use super::EmitState;
use greybel_documents::ModuleId;

impl<'a> EmitState<'a> {
    pub(crate) fn unwrap_from_parens(&self, mut id: NodeId) -> NodeId {
        while let NodeKind::Parenthesis(node) = self.kind(id) {
            id = node.expression;
        }

        id
    }

    pub(crate) fn unwrap_value(&self, id: NodeId) -> NodeId {
        match self.kind(id) {
            NodeKind::ListItem(node) => node.value,
            NodeKind::MapEntry(node) => node.value,
            _ => id,
        }
    }

    #[must_use]
    pub fn dependency_module(&self, id: NodeId) -> Option<ModuleId> {
        if self.options().dev_mode {
            return None;
        }

        self.module()?.dependency(id)
    }

    pub(crate) fn deferred_function(&self, item: NodeId, container: NodeId) -> Option<NodeId> {
        let value = self.unwrap_value(item);
        let NodeKind::Function(function) = self.kind(value) else {
            return None;
        };
        let dev_mode = self.options().dev_mode;
        let spans_lines = self.end_line(value) > self.end_line(container);

        match function.arrow {
            Some(ArrowBody::Expression) if dev_mode => None,
            Some(_) if !dev_mode => Some(value),
            _ => spans_lines.then_some(value),
        }
    }

    pub(crate) fn expression_default(&self, parameter: NodeId) -> Option<&'a Assignment> {
        let NodeKind::Assignment(assignment) = self.kind(parameter) else {
            return None;
        };

        (!self.options().dev_mode
            && !Function::is_literal_default(self.ast().nodes(), assignment.init))
        .then_some(assignment)
    }

    pub(crate) fn emitted_parameters(&self, parameters: &[NodeId]) -> Vec<NodeId> {
        parameters
            .iter()
            .map(|&parameter| {
                self.expression_default(parameter)
                    .map_or(parameter, |assignment| assignment.variable)
            })
            .collect()
    }

    pub(crate) fn has_spread(&self, fields: NodeList) -> bool {
        self.ast()
            .list(fields)
            .iter()
            .any(|&field| matches!(self.kind(field), NodeKind::Spread(_)))
    }

    pub(crate) fn is_operand(&self, node: NodeId) -> bool {
        matches!(
            self.kind(node),
            NodeKind::Identifier(_)
                | NodeKind::Member(_)
                | NodeKind::Index(_)
                | NodeKind::Call(_)
                | NodeKind::List(_)
                | NodeKind::Map(_)
                | NodeKind::Parenthesis(_)
                | NodeKind::Literal(_)
        )
    }
}
