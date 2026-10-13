use std::sync::OnceLock;

use super::rules::{LexerRuleTable, MultilineStringPolicy, ParameterDefaultPolicy};
use super::{Dialect, Version};
use crate::ast::{NodeId, NodeKind, NodeStore};
use crate::lexer::{Lexicon, Punct, punct};
use crate::parser::RuleIndex;
use crate::syntax::UnaryOp;

#[derive(Debug)]
pub struct Grammar {
    dialect: Dialect,
    version: Version,
    strict: bool,
    pub(crate) lexicon: Lexicon,
    pub(crate) rules: RuleIndex,
    pub(crate) lexer: LexerRuleTable,
    pub(crate) relaxed_separators: bool,
    pub(crate) parameter_defaults: ParameterDefaultPolicy,
    pub(crate) single_line_loops: bool,
    pub(crate) checks_open_blocks_per_function: bool,
    pub(crate) trailing_call_comma: bool,
    pub(crate) functions_in_single_line_if: bool,
    pub(crate) multiline_strings: bool,
    pub(crate) single_line_else_if: bool,
    pub(crate) return_before_else: bool,
    pub(crate) break_outside_loop_is_unmatched_block: bool,
    pub(crate) explains_single_line_if_errors: bool,
}

impl Grammar {
    /// Returns the grammar for `dialect` at `version`. Strict mode accepts only syntax compiled by the dialect's
    /// reference implementation.
    #[must_use]
    pub fn get(dialect: Dialect, version: Version, strict: bool) -> &'static Self {
        const VERSIONS: usize = Version::ALL.len();
        static GRAMMARS: [[[OnceLock<Grammar>; 2]; VERSIONS]; Dialect::ALL.len()] =
            [const { [const { [const { OnceLock::new() }; 2] }; VERSIONS] }; Dialect::ALL.len()];

        GRAMMARS[dialect as usize][version as usize][usize::from(strict)]
            .get_or_init(|| Self::new(dialect, version, strict))
    }

    fn new(dialect: Dialect, version: Version, strict: bool) -> Self {
        let rules = dialect.rules(strict);
        let mut punctuators: Vec<&[Punct]> = rules.punctuators.to_vec();

        if strict && version.compound_assignments() {
            punctuators.push(punct::COMPOUND_ASSIGNMENTS);
        }

        Self {
            dialect,
            version,
            strict,
            lexicon: Lexicon::new(rules.lexer, rules.keywords, &punctuators),
            rules: RuleIndex::new(rules, (rules.precedence)(version)),
            lexer: rules.lexer,
            relaxed_separators: rules.relaxed_separators,
            parameter_defaults: rules.parameter_defaults,
            single_line_loops: rules.single_line_loops,
            checks_open_blocks_per_function: rules.checks_open_blocks_per_function,
            trailing_call_comma: rules.trailing_call_comma,
            functions_in_single_line_if: rules.functions_in_single_line_if,
            multiline_strings: match rules.multiline_strings {
                MultilineStringPolicy::Always => true,
                MultilineStringPolicy::Forbidden => false,
                MultilineStringPolicy::ByVersion => !version.strings_end_at_line_break(),
            },
            single_line_else_if: version.single_line_else_if(),
            return_before_else: version.return_before_else(),
            break_outside_loop_is_unmatched_block: version.break_outside_loop_is_unmatched_block(),
            explains_single_line_if_errors: version.explains_single_line_if_errors(),
        }
    }

    pub(crate) fn accepts_default(&self, nodes: &NodeStore, default: NodeId) -> bool {
        let node = nodes.get(default);

        match (&node.kind, self.parameter_defaults) {
            (NodeKind::Literal(_), _) => true,
            (
                NodeKind::Parenthesis(parenthesis),
                ParameterDefaultPolicy::ParenthesizedLiterals
                | ParameterDefaultPolicy::LiteralOrReference,
            ) => self.accepts_default(nodes, parenthesis.expression),
            (NodeKind::Unary(unary), ParameterDefaultPolicy::LiteralOrReference) => {
                unary.operator == UnaryOp::AddressOf
                    && matches!(
                        nodes.get(unary.argument).kind,
                        NodeKind::Identifier(_) | NodeKind::Member(_)
                    )
            }
            _ => false,
        }
    }

    #[must_use]
    pub fn dialect(&self) -> Dialect {
        self.dialect
    }

    #[must_use]
    pub fn version(&self) -> Version {
        self.version
    }

    #[must_use]
    pub fn is_strict(&self) -> bool {
        self.strict
    }
}

#[cfg(test)]
#[path = "tests/grammar.rs"]
mod tests;
