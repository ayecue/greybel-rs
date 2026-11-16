pub mod directives;
mod expansion;
pub mod operators;
pub mod statements;
mod template;
mod type_syntax;

pub use directives::*;
pub use expansion::ValueExpansions;
pub use operators::*;
pub use statements::*;
pub use template::Template;
pub use type_syntax::TypeSyntax;

use crate::parser::{InfixRule, KeywordStatement, PrefixRule};

pub struct GreybelRuleTable;

impl GreybelRuleTable {
    pub const INFIX: &'static [&'static dyn InfixRule] = &[
        &TernaryRule,
        &CoalesceRule,
        &OptionalChainRule,
        &NonNullRule,
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
        &EndSwitchRule,
        &EndClassRule,
    ];
}
