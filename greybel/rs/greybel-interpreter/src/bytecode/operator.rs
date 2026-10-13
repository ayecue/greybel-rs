use greybel_core::{BinaryOp, BitwiseOp};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Operator {
    Binary(BinaryOp),
    Bitwise(BitwiseOp),
    Isa,
    And,
    Or,
}

impl From<BinaryOp> for Operator {
    fn from(operator: BinaryOp) -> Self {
        Self::Binary(operator)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOperator {
    Not,
    Negate,
    New,
    MiniScriptNew,
}
