use super::rules::{LexerRuleTable, MultilineStringPolicy, ParameterDefaultPolicy, RuleTable};
use super::{SyntaxFeatures, Version};
use crate::lexer::{keyword, punct};
use crate::syntax::Core;

pub(super) const RULES: RuleTable = RuleTable {
    keywords: &[keyword::KEYWORDS],
    punctuators: &[punct::PUNCTUATORS, punct::COMPOUND_ASSIGNMENTS],
    statements: &[Core::STATEMENTS],
    prefix: &[Core::PREFIX],
    infix: &[Core::INFIX],
    precedence: Version::precedence,
    lexer: LexerRuleTable {
        block_comments: false,
        commented_directives: false,
        hash_identifiers: false,
    },
    relaxed_separators: false,
    parameter_defaults: ParameterDefaultPolicy::Literals,
    single_line_loops: false,
    checks_open_blocks_per_function: false,
    trailing_call_comma: false,
    functions_in_single_line_if: false,
    multiline_strings: MultilineStringPolicy::ByVersion,
    features: SyntaxFeatures::NONE,
};

pub(super) const STRICT_RULES: RuleTable = RuleTable {
    punctuators: &[punct::PUNCTUATORS],
    ..RULES
};
