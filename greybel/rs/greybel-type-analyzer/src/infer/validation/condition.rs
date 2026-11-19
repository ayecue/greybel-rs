use greybel_core::{BinaryOp, LiteralValue, LogicalOp, NodeId, NodeKind, UnaryOp};
use greybel_symbols::literals::{FALSE, TRUE};

use crate::diagnostic::DiagnosticKind;
use crate::infer::Inference;

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
    pub(crate) fn validate_condition(&mut self, condition: NodeId) {
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
            NodeKind::Binary(_) | NodeKind::Logical(_) => self.constant_chain(node),
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
            _ => None,
        }
    }

    fn constant_chain(&self, node: NodeId) -> Option<Constant> {
        let ast = self.ast();
        let chain = ast.operator_chain(node);
        let mut left = self.constant_value(chain.first)?;

        for &link in &chain.links {
            left = match ast.kind(link) {
                NodeKind::Binary(binary) => {
                    left.compare(binary.operator, &self.constant_value(binary.right)?)?
                }
                NodeKind::Logical(logical) => {
                    let right = self.constant_value(logical.right)?;

                    if !left.is_boolean_like() || !right.is_boolean_like() {
                        return None;
                    }

                    Constant::from_bool(match logical.operator {
                        LogicalOp::And => left.is_truthy() && right.is_truthy(),
                        LogicalOp::Or => left.is_truthy() || right.is_truthy(),
                    })
                }
                _ => return None,
            };
        }

        Some(left)
    }

    pub(crate) fn is_always_true(&self, condition: NodeId) -> bool {
        self.constant_value(condition)
            .is_some_and(|constant| constant.is_truthy())
    }
}
