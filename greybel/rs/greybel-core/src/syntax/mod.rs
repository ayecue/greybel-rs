mod chunk;
pub mod expressions;
pub mod greybel;
pub mod greyscript;
pub mod statements;
mod system;

pub use chunk::{Chunk, PendingChunk};
pub use expressions::*;
pub use statements::*;
pub use system::{Empty, Invalid, Noop, Unknown};

use crate::parser::{InfixRule, KeywordStatement, PrefixRule};

pub struct Core;

impl Core {
    pub const INFIX: &'static [&'static dyn InfixRule] = &[
        &MemberRule,
        &IndexRule,
        &CallRule,
        &BinaryRule::POWER,
        &BinaryRule::MULTIPLY,
        &BinaryRule::DIVIDE,
        &BinaryRule::MODULO,
        &BinaryRule::ADD,
        &SubtractRule,
        &ComparisonRule,
        &IsaRule,
        &LogicalRule::AND,
        &LogicalRule::OR,
    ];
    pub const PREFIX: &'static [&'static dyn PrefixRule] = &[
        &UnaryRule::ADDRESS_OF,
        &UnaryRule::NEW,
        &UnaryRule::NEGATE,
        &UnaryRule::NOT,
        &MapRule,
        &ListRule,
        &ParenthesisRule,
    ];
    pub const STATEMENTS: &'static [&'static dyn KeywordStatement] = &[
        &ReturnRule,
        &IfRule,
        &ElseRule,
        &EndIfRule,
        &WhileRule,
        &EndLoopRule::<While>::RULE,
        &ForRule,
        &EndLoopRule::<For>::RULE,
        &EndFunctionRule,
        &LoopJumpRule,
        &MisplacedKeywordRule::FUNCTION,
        &MisplacedKeywordRule::NEW,
        &MisplacedKeywordRule::AND,
        &MisplacedKeywordRule::OR,
        &MisplacedKeywordRule::ISA,
        &MisplacedKeywordRule::IN,
        &MisplacedKeywordRule::THEN,
        &MisplacedKeywordRule::REPEAT,
    ];
}
