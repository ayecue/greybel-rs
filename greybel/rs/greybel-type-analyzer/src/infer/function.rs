use std::mem;

use greybel_core::ast::NodeFieldContainer;
use greybel_core::{NodeId, NodeKind};
use greybel_symbols::annotations::Tag;
use greybel_symbols::names::SELF;

use super::Inference;
use crate::annotation::TypeExpr;
use crate::model::{FunctionId, ScopeId, SlotId};
use crate::types::TypeId;

impl Inference<'_> {
    pub(crate) fn infer_function_body(&mut self, node: NodeId) -> FunctionId {
        let ast = self.ast();
        let function = self.function_id(node);
        let NodeKind::Function(declaration) = ast.kind(node) else {
            unreachable!("functions are function nodes")
        };
        let previous = mem::replace(&mut self.current_scope, ScopeId::Function(function));
        let outer_body = self.enter_body();

        self.begin_side_effect(Some(ScopeId::Function(function)));

        let data = self.model.function(function);
        let (receiver, returns) = (data.receiver_slot, data.return_slot);
        let doc = data.doc.clone();
        let parameters: Vec<_> = data
            .parameters
            .iter()
            .map(|parameter| {
                let annotation = doc.as_ref().and_then(|doc| {
                    doc.params
                        .iter()
                        .position(|param| param.name == parameter.name)
                });

                (
                    parameter.node,
                    parameter.slot,
                    parameter.name == SELF,
                    annotation,
                )
            })
            .collect();
        let arguments = self.model.bound_arguments(function).map(<[_]>::to_vec);
        let depth = self.type_parameters_in_scope.len();
        let parameter_slots: Vec<SlotId> = parameters.iter().map(|&(_, slot, ..)| slot).collect();

        self.unassign_variables(ScopeId::Function(function), &parameter_slots);

        if let Some(doc) = &doc {
            let templates = self.template_parameters(&doc.templates);

            self.type_parameters_in_scope.extend(templates);
        }

        for (position, (parameter, slot, is_self, annotation)) in parameters.into_iter().enumerate()
        {
            let argument = arguments
                .as_ref()
                .and_then(|arguments| arguments.get(position).copied().flatten());
            let inferred_type = match (argument, ast.kind(parameter)) {
                (Some(_), _) => self.model.slot_type(slot),
                (None, NodeKind::Assignment(assignment)) => self.infer_value(assignment.init),
                (None, _) if position == 0 && is_self => self.model.slot_type(receiver),
                (None, _) if arguments.is_some() => TypeId::NULL,
                (None, _) => TypeId::ANY,
            };
            let annotation_type = annotation
                .and_then(|index| doc.as_ref()?.params[index].ty.as_ref())
                .map(|annotation_type| self.resolve_signature_type(annotation_type));
            let value = match annotation_type {
                Some(annotation_type) if annotation_type != TypeId::UNKNOWN => {
                    if let NodeKind::Assignment(assignment) = ast.kind(parameter)
                        && let Some(name) = self.identifier_name(assignment.variable)
                    {
                        self.validate_against_annotation(
                            parameter,
                            Tag::Param,
                            name,
                            annotation_type,
                            inferred_type,
                        );
                    }

                    annotation_type
                }
                _ => inferred_type,
            };

            self.widen(slot, value);

            let ty = self.model.slot_type(slot);
            let variable = match ast.kind(parameter) {
                NodeKind::Assignment(assignment) => assignment.variable,
                _ => parameter,
            };

            self.add_definition(variable, &[slot]);
            if let NodeKind::Assignment(assignment) = ast.kind(parameter) {
                self.register_node_type(assignment.variable, ty);
            }

            self.register_node_type(parameter, ty);
        }

        self.infer_block(ast.list(declaration.body));

        let annotated_returns = doc.as_ref().and_then(|doc| doc.returns.as_ref());
        let returns_nothing = ast
            .kind(node)
            .as_scope()
            .is_none_or(|scope| scope.scope().returns.is_empty());
        let reaches_end = self.flow.is_reachable();

        if let Some(annotation_type) = annotated_returns {
            let annotation_type = self.resolve_signature_type(annotation_type);

            self.widen(returns, annotation_type);
        } else if reaches_end {
            self.widen(returns, TypeId::NULL);
        } else if returns_nothing {
            self.widen(returns, TypeId::NEVER);
        }

        self.type_parameters_in_scope.truncate(depth);
        self.end_function_side_effect(self.model.function(function).site);
        self.exit_body(outer_body);
        self.current_scope = previous;

        function
    }

    pub(super) fn resolve_signature_type(&mut self, expr: &TypeExpr) -> TypeId {
        self.implicit_generics = true;

        let ty = self.resolve_type(expr);

        self.implicit_generics = false;

        ty
    }
}
