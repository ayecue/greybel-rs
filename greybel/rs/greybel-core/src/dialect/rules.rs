use super::{SyntaxFeatures, Version};
use crate::lexer::{Keyword, Punct};
use crate::parser::{InfixRule, KeywordStatement, PrecedenceTable, PrefixRule, RuleTableSet};

pub(crate) struct RuleTable {
    pub keywords: &'static [&'static [Keyword]],
    pub punctuators: &'static [&'static [Punct]],
    pub statements: RuleTableSet<dyn KeywordStatement>,
    pub prefix: RuleTableSet<dyn PrefixRule>,
    pub infix: RuleTableSet<dyn InfixRule>,
    pub precedence: fn(Version) -> PrecedenceTable,
    pub lexer: LexerRuleTable,
    /// Permits line breaks and comments before call and literal separators outside strict mode.
    pub relaxed_separators: bool,
    /// Selects the expressions accepted as function parameter defaults.
    pub parameter_defaults: ParameterDefaultPolicy,
    /// Permits a loop body on the `while` or `for` header line.
    pub single_line_loops: bool,
    /// Enables Grey Hack's per-function open-block check (`Parser.CheckForOpenBackpatches`). `end function`
    /// reports an unclosed block at its opening line; an unclosed `if` reports missing `end if`, including before
    /// `else`.
    pub checks_open_blocks_per_function: bool,
    /// Permits trailing commas in call argument lists.
    pub trailing_call_comma: bool,
    /// Permits a single-line `if` to contain a function definition ending on a later line.
    pub functions_in_single_line_if: bool,
    /// Selects whether string literals may contain line breaks.
    pub multiline_strings: MultilineStringPolicy,
    /// The syntax features the grammar accepts.
    pub features: SyntaxFeatures,
}

/// Selects the expressions accepted as function parameter defaults.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ParameterDefaultPolicy {
    /// Accepts literal defaults only.
    Literals,
    /// Accepts literals and parenthesized literals. Grey Hack's `Parser.ParseFunction` rejects defaults evaluated
    /// to temporaries (`ValTemp`). Although Grey Hack accepts `@` references, it stores the unevaluated reference
    /// as `ValVar`; strict mode therefore rejects them.
    ParenthesizedLiterals,
    /// Accepts any expression. A default that isn't a literal is evaluated at the start of each call that omits the
    /// argument (or passes `null`); the transpiler writes it as `if @name == null then name = default`.
    Expression,
}

/// Selects whether string literals may contain line breaks.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MultilineStringPolicy {
    /// Permits multiline strings in every dialect version.
    Always,
    /// Forbids multiline strings; a line break terminates an unclosed string, matching Grey Hack 0.9.7373 and later.
    Forbidden,
    /// Delegates multiline-string support to [`Version::strings_end_at_line_break`].
    ByVersion,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct LexerRuleTable {
    /// Enables C-style `/* ... */` comments.
    pub block_comments: bool,
    /// Treats `//import` and `//include` as directives, not comments.
    pub commented_directives: bool,
    /// Permits `#` before directive keywords.
    pub hash_identifiers: bool,
}
