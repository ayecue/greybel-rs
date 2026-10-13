pub mod directives;
pub mod operators;
pub mod statements;

pub use directives::*;
pub use operators::*;
pub use statements::*;

use crate::parser::{InfixRule, KeywordStatement, PrefixRule};

pub struct GreybelRuleTable;

impl GreybelRuleTable {
    pub const INFIX: &'static [&'static dyn InfixRule] = &[
        &BitwiseRule::OR,
        &BitwiseRule::AND,
        &BitwiseRule::SHIFT_LEFT,
        &BitwiseRule::SHIFT_RIGHT,
        &BitwiseRule::UNSIGNED_SHIFT_RIGHT,
    ];
    pub const PREFIX: &'static [&'static dyn PrefixRule] = &[
        &EnvarExpressionRule,
        &InjectExpressionRule,
        &LineRule,
        &FileRule,
    ];
    pub const STATEMENTS: &'static [&'static dyn KeywordStatement] = &[
        &IncludeRule,
        &ImportRule,
        &EnvarStatementRule,
        &InjectStatementRule,
        &DebuggerRule,
        &IfEnvRule,
        &ElseEnvRule,
        &EndIfEnvRule,
    ];
}
