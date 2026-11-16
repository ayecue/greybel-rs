use greybel_core::{BinaryOp, NodeId, NodeKind};

use super::MessagePart;
use crate::diagnostic::DiagnosticKind;
use crate::infer::Inference;
use crate::types::{LiteralType, Type, TypeId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Operand {
    Number,
    String,
    List,
    Map,
    Function,
    Null,
}

impl Operand {
    pub(crate) fn of(ty: &Type) -> Option<Self> {
        Some(match ty {
            Type::Number | Type::Literal(LiteralType::Number(_)) => Self::Number,
            Type::String | Type::Literal(LiteralType::String(_)) => Self::String,
            Type::List(_) => Self::List,
            Type::Map { .. } | Type::Object(_) | Type::Class(_) => Self::Map,
            Type::Function(_) => Self::Function,
            Type::Null => Self::Null,
            Type::Any
            | Type::Unknown
            | Type::Never
            | Type::Union(_)
            | Type::TypeParameter { .. } => {
                return None;
            }
        })
    }

    pub(crate) const fn is_number_or_null(self) -> bool {
        matches!(self, Self::Number | Self::Null)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Evaluation {
    Value,
    Null,
    TypeMismatch,
}

#[derive(Debug, Clone, Copy)]
struct Failure {
    null: bool,
    type_mismatch: bool,
}

impl Evaluation {
    pub(crate) fn of_binary(operator: BinaryOp, left: Operand, right: Operand) -> Self {
        let arithmetic = if left == Operand::Number && right.is_number_or_null() {
            Self::Value
        } else {
            Self::Null
        };

        match operator {
            BinaryOp::Equal | BinaryOp::NotEqual => Self::Value,
            BinaryOp::Add => match (left, right) {
                (Operand::String, _)
                | (_, Operand::String)
                | (Operand::List, Operand::List)
                | (Operand::Map, Operand::Map) => Self::Value,
                (Operand::List | Operand::Map, _) => Self::TypeMismatch,
                _ => arithmetic,
            },
            BinaryOp::Subtract => match (left, right) {
                (Operand::String, Operand::String | Operand::Null) => Self::Value,
                _ => arithmetic,
            },
            BinaryOp::Multiply | BinaryOp::Divide => match (left, right) {
                (Operand::String | Operand::List, Operand::Number) => Self::Value,
                (Operand::String | Operand::List, _) => Self::TypeMismatch,
                _ => arithmetic,
            },
            BinaryOp::Modulo | BinaryOp::Power => arithmetic,
            BinaryOp::Less | BinaryOp::LessEqual | BinaryOp::Greater | BinaryOp::GreaterEqual => {
                match (left, right) {
                    (Operand::String, Operand::String) => Self::Value,
                    _ => arithmetic,
                }
            }
        }
    }
}

impl Inference<'_> {
    pub(crate) fn validate_binary(
        &mut self,
        node: NodeId,
        operator: BinaryOp,
        (left_node, left): (NodeId, TypeId),
        (right_node, right): (NodeId, TypeId),
    ) {
        let Some(operation_failure) = self.operation_failure(left, right, |left, right| {
            Evaluation::of_binary(operator, left, right)
        }) else {
            return;
        };

        self.report_operation(
            node,
            operator.as_str(),
            &[(left_node, left), (right_node, right)],
            operation_failure,
        );
    }

    pub(crate) fn validate_bitwise(
        &mut self,
        node: NodeId,
        symbol: &str,
        (left_node, left): (NodeId, TypeId),
        (right_node, right): (NodeId, TypeId),
    ) {
        let Some(operation_failure) = self.operation_failure(left, right, |left, right| {
            if left == Operand::Number && right.is_number_or_null() {
                Evaluation::Value
            } else {
                Evaluation::Null
            }
        }) else {
            return;
        };

        self.report_operation(
            node,
            symbol,
            &[(left_node, left), (right_node, right)],
            operation_failure,
        );
    }

    pub(crate) fn validate_negation(
        &mut self,
        node: NodeId,
        (operand_node, operand): (NodeId, TypeId),
    ) {
        let Some(operation_failure) =
            self.operation_failure(operand, TypeId::NULL, |operand, _| {
                if operand.is_number_or_null() {
                    Evaluation::Value
                } else {
                    Evaluation::Null
                }
            })
        else {
            return;
        };

        self.report_operation(node, "-", &[(operand_node, operand)], operation_failure);
    }

    fn operation_failure(
        &self,
        left: TypeId,
        right: TypeId,
        evaluation: impl Fn(Operand, Operand) -> Evaluation,
    ) -> Option<Failure> {
        if self.context.current.is_some() {
            return None;
        }

        let is_known = |ty: TypeId| {
            self.types.variants(ty).next().is_some()
                && self
                    .types
                    .variants(ty)
                    .all(|variant| Operand::of(self.types.get(variant)).is_some())
        };

        if !is_known(left) || !is_known(right) {
            return None;
        }

        let mut operation_failure = Failure {
            null: false,
            type_mismatch: false,
        };

        for left in self.types.variants(left) {
            let left = Operand::of(self.types.get(left))?;

            for right in self.types.variants(right) {
                match evaluation(left, Operand::of(self.types.get(right))?) {
                    Evaluation::Value => return None,
                    Evaluation::Null => operation_failure.null = true,
                    Evaluation::TypeMismatch => operation_failure.type_mismatch = true,
                }
            }
        }

        Some(operation_failure)
    }

    fn report_operation(
        &mut self,
        node: NodeId,
        symbol: &str,
        operands: &[(NodeId, TypeId)],
        operation_failure: Failure,
    ) {
        let site = self.site(node);
        let follows_report = operands.iter().any(|&(operand, _)| {
            let site = self.site(self.without_parentheses(operand));

            self.pass.invalid_operations.contains(&site)
        });

        self.pass.invalid_operations.insert(site);

        if follows_report {
            return;
        }

        let mut message = vec![MessagePart::Text(format!("\"{symbol}\" on "))];

        for (index, &(_, operand)) in operands.iter().enumerate() {
            if index > 0 {
                message.push(MessagePart::Text(" and ".to_owned()));
            }

            message.push(MessagePart::Type(operand));
        }

        message.push(MessagePart::Text(
            match (operation_failure.null, operation_failure.type_mismatch) {
                (true, true) => " gives null or raises a type mismatch.",
                (false, true) => " raises a type mismatch.",
                _ => " always gives null.",
            }
            .to_owned(),
        ));

        self.defer_diagnostic(DiagnosticKind::InvalidOperation, node, message);
    }

    fn without_parentheses(&self, mut node: NodeId) -> NodeId {
        while let NodeKind::Parenthesis(parenthesis) = self.ast().kind(node) {
            node = parenthesis.expression;
        }

        node
    }
}
