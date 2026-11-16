use crate::vocabulary::vocabulary;

vocabulary!(Precedence {
    Ternary => "ternary",
    Coalesce => "coalesce",
    Or => "or",
    And => "and",
    Not => "not",
    Isa => "isa",
    Comparison => "comparison",
    BitwiseOr => "bitwise or",
    BitwiseAnd => "bitwise and",
    AddSub => "add/sub",
    Shift => "shift",
    MulDiv => "mul/div",
    UnaryMinus => "unary minus",
    New => "new",
    AddressOf => "address of",
    Power => "power",
    Call => "call",
});
