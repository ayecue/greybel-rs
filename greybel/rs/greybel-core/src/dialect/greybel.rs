use super::rules::{LexerRuleTable, MultilineStringPolicy, ParameterDefaultPolicy, RuleTable};
use super::{SyntaxFeatures, Version};
use crate::lexer::{self, keyword, punct};
use crate::parser::{Precedence, PrecedenceTable};
use crate::syntax::Core;
use crate::syntax::greybel::GreybelRuleTable;

pub(super) const RULES: RuleTable = RuleTable {
    keywords: &[keyword::KEYWORDS, lexer::greybel::KEYWORDS],
    punctuators: &[
        punct::PUNCTUATORS,
        punct::COMPOUND_ASSIGNMENTS,
        lexer::greybel::PUNCTUATORS,
    ],
    statements: &[Core::STATEMENTS, GreybelRuleTable::STATEMENTS],
    prefix: &[Core::PREFIX, GreybelRuleTable::PREFIX],
    infix: &[Core::INFIX, GreybelRuleTable::INFIX],
    precedence: precedence_table,
    lexer: LEXER,
    relaxed_separators: true,
    parameter_defaults: ParameterDefaultPolicy::Expression,
    single_line_loops: true,
    checks_open_blocks_per_function: true,
    trailing_call_comma: true,
    functions_in_single_line_if: true,
    multiline_strings: MultilineStringPolicy::Always,
    features: SyntaxFeatures::ALL,
};

pub(super) const STRICT_RULES: RuleTable = RuleTable {
    punctuators: &[punct::PUNCTUATORS, lexer::greybel::PUNCTUATORS],
    lexer: LexerRuleTable { ..LEXER },
    relaxed_separators: false,
    parameter_defaults: ParameterDefaultPolicy::ParenthesizedLiterals,
    single_line_loops: false,
    checks_open_blocks_per_function: true,
    trailing_call_comma: false,
    functions_in_single_line_if: false,
    multiline_strings: MultilineStringPolicy::ByVersion,
    features: SyntaxFeatures::NONE,
    ..RULES
};

pub(super) const LEXER: LexerRuleTable = LexerRuleTable {
    block_comments: true,
    commented_directives: true,
    hash_identifiers: true,
};

pub(super) fn precedence_table(version: Version) -> PrecedenceTable {
    version
        .precedence()
        .insert_after(
            Precedence::Isa,
            &[Precedence::BitwiseOr, Precedence::BitwiseAnd],
        )
        .insert_after(Precedence::AddSub, &[Precedence::Shift])
        .insert_before(
            Precedence::Or,
            &[Precedence::Ternary, Precedence::Coalesce],
        )
}
