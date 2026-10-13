use super::Version;
use super::greybel::{LEXER, precedence_table};
use super::rules::{LexerRuleTable, MultilineStringPolicy, ParameterDefaultPolicy, RuleTable};
use crate::lexer::{self, keyword, punct};
use crate::syntax::Core;
use crate::syntax::greybel::GreybelRuleTable;
use crate::syntax::greyscript::GreyScriptRuleTable;

pub(super) const RULES: RuleTable = RuleTable {
    keywords: &[
        keyword::KEYWORDS,
        lexer::greybel::KEYWORDS,
        lexer::greyscript::KEYWORDS,
    ],
    punctuators: &[
        punct::PUNCTUATORS,
        punct::COMPOUND_ASSIGNMENTS,
        lexer::greybel::PUNCTUATORS,
    ],
    statements: &[
        Core::STATEMENTS,
        GreybelRuleTable::STATEMENTS,
        GreyScriptRuleTable::STATEMENTS,
    ],
    prefix: &[Core::PREFIX, GreybelRuleTable::PREFIX],
    infix: &[Core::INFIX, GreybelRuleTable::INFIX],
    precedence: precedence_table,
    lexer: LEXER,
    relaxed_separators: true,
    parameter_defaults: ParameterDefaultPolicy::ParenthesizedLiterals,
    single_line_loops: true,
    checks_open_blocks_per_function: true,
    trailing_call_comma: true,
    functions_in_single_line_if: true,
    multiline_strings: MultilineStringPolicy::Always,
};

pub(super) const STRICT_RULES: RuleTable = RuleTable {
    keywords: &[keyword::KEYWORDS, lexer::greyscript::KEYWORDS],
    punctuators: &[punct::PUNCTUATORS],
    statements: &[Core::STATEMENTS, GreyScriptRuleTable::STATEMENTS],
    prefix: &[Core::PREFIX],
    infix: &[Core::INFIX],
    precedence: Version::precedence,
    lexer: LexerRuleTable {
        block_comments: false,
        commented_directives: false,
        hash_identifiers: false,
    },
    relaxed_separators: false,
    parameter_defaults: ParameterDefaultPolicy::ParenthesizedLiterals,
    single_line_loops: false,
    checks_open_blocks_per_function: true,
    trailing_call_comma: false,
    functions_in_single_line_if: false,
    multiline_strings: MultilineStringPolicy::Forbidden,
};
