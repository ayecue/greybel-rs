use greybel_core::{BinaryOp, LiteralValue, LogicalOp, NodeId, NodeKind, UnaryOp};
use greybel_symbols::literals::{FALSE, TRUE};

use super::Inference;
use crate::diagnostic::DiagnosticKind;
use crate::model::{DocumentIndex, ScopeId, Site, SlotId};

#[derive(Debug, Clone, Copy)]
pub(super) struct GlobalRead {
    document: DocumentIndex,
    node: NodeId,
    slot: SlotId,
    outermost_loop: Option<Site>,
}

#[derive(Debug, Clone, PartialEq)]
enum Constant {
    Number(f64),
    String(String),
    Null,
}

impl Constant {
    fn is_truthy(&self) -> bool {
        match self {
            Self::Number(number) => *number != 0.0,
            Self::String(text) => !text.is_empty(),
            Self::Null => false,
        }
    }

    #[expect(
        clippy::float_cmp,
        reason = "Only the exact numeric constants zero and one are boolean-like"
    )]
    fn is_boolean_like(&self) -> bool {
        match self {
            Self::Number(number) => *number == 0.0 || *number == 1.0,
            Self::String(_) | Self::Null => true,
        }
    }

    fn from_bool(value: bool) -> Self {
        Self::Number(f64::from(u8::from(value)))
    }

    fn compare(&self, operator: BinaryOp, other: &Self) -> Option<Self> {
        let order = match (self, other) {
            (Self::Number(left), Self::Number(right)) => left.partial_cmp(right),
            (Self::String(left), Self::String(right)) => Some(left.cmp(right)),
            (Self::Null, Self::Null) => Some(std::cmp::Ordering::Equal),
            _ => None,
        };
        let result = match (operator, order) {
            (BinaryOp::Equal, order) => order.is_some_and(std::cmp::Ordering::is_eq),
            (BinaryOp::NotEqual, order) => !order.is_some_and(std::cmp::Ordering::is_eq),
            (_, Some(_)) if matches!(self, Self::Null) => return None,
            (BinaryOp::Less, Some(order)) => order.is_lt(),
            (BinaryOp::Greater, Some(order)) => order.is_gt(),
            (BinaryOp::LessEqual, Some(order)) => order.is_le(),
            (BinaryOp::GreaterEqual, Some(order)) => order.is_ge(),
            _ => return None,
        };

        Some(Self::from_bool(result))
    }
}

impl Inference<'_> {
    pub(super) fn validate_condition(&mut self, condition: NodeId) {
        if let Some(constant) = self.constant_value(condition) {
            let value = if constant.is_truthy() { TRUE } else { FALSE };

            self.report(
                DiagnosticKind::ConstantCondition,
                condition,
                format!("Constant condition is always {value}."),
            );
        }
    }

    fn constant_value(&self, node: NodeId) -> Option<Constant> {
        let ast = self.ast();

        match ast.kind(node) {
            NodeKind::Literal(literal) => Some(match &literal.value {
                LiteralValue::Number(number) if literal.negated => Constant::Number(-number),
                LiteralValue::Number(number) => Constant::Number(*number),
                LiteralValue::Boolean(value) => Constant::from_bool(*value),
                LiteralValue::String(text) => Constant::String(ast.read(*text).into_owned()),
                LiteralValue::Nil => Constant::Null,
            }),
            NodeKind::Parenthesis(parenthesis) => self.constant_value(parenthesis.expression),
            NodeKind::Unary(unary) if unary.operator == UnaryOp::Not => {
                Some(match self.constant_value(unary.argument)? {
                    Constant::Number(number) => Constant::Number(1.0 - number.abs().min(1.0)),
                    Constant::String(text) => Constant::from_bool(text.is_empty()),
                    Constant::Null => Constant::Number(1.0),
                })
            }
            NodeKind::Binary(binary) => {
                let left = self.constant_value(binary.left)?;
                let right = self.constant_value(binary.right)?;

                left.compare(binary.operator, &right)
            }
            NodeKind::ComparisonGroup(group) => {
                let operands = ast
                    .list(group.expressions)
                    .iter()
                    .map(|&operand| self.constant_value(operand))
                    .collect::<Option<Vec<_>>>()?;
                let mut all = true;

                for (operator, pair) in group.operators.iter().zip(operands.windows(2)) {
                    all &= pair[0].compare(*operator, &pair[1])?.is_truthy();
                }

                Some(Constant::from_bool(all))
            }
            NodeKind::Logical(logical) => {
                let left = self.constant_value(logical.left)?;
                let right = self.constant_value(logical.right)?;

                if !left.is_boolean_like() || !right.is_boolean_like() {
                    return None;
                }

                Some(Constant::from_bool(match logical.operator {
                    LogicalOp::And => left.is_truthy() && right.is_truthy(),
                    LogicalOp::Or => left.is_truthy() || right.is_truthy(),
                }))
            }
            _ => None,
        }
    }

    pub(super) fn validate_self_assignment(&mut self, target: NodeId, init: NodeId) {
        let (Some(name), Some(read)) = (self.identifier_name(target), self.identifier_name(init))
        else {
            return;
        };

        if name != read
            || !matches!(self.ast().kind(target), NodeKind::Identifier(_))
            || !matches!(self.ast().kind(init), NodeKind::Identifier(_))
        {
            return;
        }

        let Some(slot) = self.model.variable(self.current_scope, name) else {
            return;
        };

        if self.resolve_variable(name) != Some(slot) || !self.is_defined_before(slot, target) {
            return;
        }

        self.report(
            DiagnosticKind::SelfAssignment,
            target,
            format!("Self-assignment: {name}"),
        );
    }

    fn is_defined_before(&self, slot: SlotId, node: NodeId) -> bool {
        let ast = self.ast();
        let start = ast.span(node).start;

        self.references
            .definition_sites(slot)
            .iter()
            .any(|site| site.document == self.current_document && ast.span(site.node).start < start)
    }

    pub(super) fn enter_loop(&mut self, node: NodeId) {
        self.loop_stack.push(Site {
            document: self.current_document,
            node,
        });
    }

    pub(super) fn exit_loop(&mut self) {
        self.loop_stack.pop();
    }

    pub(super) fn record_global_read(&mut self, node: NodeId, slot: SlotId) {
        if self.call_context.is_none() && self.current_scope == ScopeId::Global {
            self.pass.global_reads.push(GlobalRead {
                document: self.current_document,
                node,
                slot,
                outermost_loop: self.loop_stack.first().copied(),
            });
        }
    }

    pub(super) fn validate_definition_order(&mut self) {
        let reads = std::mem::take(&mut self.pass.global_reads);

        for read in reads {
            let ast = self.documents[read.document as usize].ast;
            let sites = self.references.definition_sites(read.slot);
            let start = ast.span(read.node).start;
            let is_repeated = read
                .outermost_loop
                .is_some_and(|outermost| outermost.document != read.document);
            let defined_in_time = is_repeated
                || sites.is_empty()
                || sites.iter().any(|site| {
                    if site.document != read.document {
                        return true;
                    }

                    let span = ast.span(site.node);

                    ast.node(site.node)
                        .scope
                        .is_some_and(|scope| matches!(ast.kind(scope), NodeKind::Function(_)))
                        || span.start < start
                        || read.outermost_loop.is_some_and(|outermost| {
                            let loop_span = ast.span(outermost.node);

                            loop_span.start <= span.start && span.end <= loop_span.end
                        })
                });

            if defined_in_time {
                continue;
            }

            let name = ast.text(read.node).unwrap_or_default().to_owned();

            self.report_in(
                read.document,
                DiagnosticKind::UseBeforeDefinition,
                read.node,
                format!("Used before defined: {name}"),
            );
        }
    }
}
