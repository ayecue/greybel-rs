use std::sync::{Mutex, OnceLock, PoisonError};

use rustc_hash::FxHashMap;

use super::rules::{LexerRuleTable, MultilineStringPolicy, ParameterDefaultPolicy};
use super::{Dialect, SyntaxFeatures, Version};
use crate::ast::{NodeId, NodeKind, NodeStore};
use crate::lexer::{Lexicon, Punct, punct};
use crate::parser::RuleIndex;

#[derive(Debug)]
pub struct Grammar {
    dialect: Dialect,
    version: Version,
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
    features: SyntaxFeatures,
}

impl Grammar {
    #[must_use]
    pub fn get(dialect: Dialect, version: Version, strict: bool) -> &'static Self {
        const VERSIONS: usize = Version::ALL.len();
        static GRAMMARS: [[[OnceLock<Grammar>; 2]; VERSIONS]; Dialect::ALL.len()] =
            [const { [const { [const { OnceLock::new() }; 2] }; VERSIONS] }; Dialect::ALL.len()];

        GRAMMARS[dialect as usize][version as usize][usize::from(strict)]
            .get_or_init(|| Self::new(dialect, version, strict, SyntaxFeatures::ALL))
    }

    #[must_use]
    pub fn with_features(
        dialect: Dialect,
        version: Version,
        strict: bool,
        features: SyntaxFeatures,
    ) -> &'static Self {
        type Key = (Dialect, Version, bool, SyntaxFeatures);

        static GRAMMARS: OnceLock<Mutex<FxHashMap<Key, &'static Grammar>>> = OnceLock::new();

        let grammar = Self::get(dialect, version, strict);
        let features = grammar.features.intersection(features);

        if features == grammar.features {
            return grammar;
        }

        let mut grammars = GRAMMARS
            .get_or_init(Mutex::default)
            .lock()
            .unwrap_or_else(PoisonError::into_inner);

        grammars
            .entry((dialect, version, strict, features))
            .or_insert_with(|| Box::leak(Box::new(Self::new(dialect, version, strict, features))))
    }

    fn new(dialect: Dialect, version: Version, strict: bool, features: SyntaxFeatures) -> Self {
        let rules = dialect.rules(strict);
        let features = rules.features.intersection(features);
        let mut punctuators: Vec<&[Punct]> = rules.punctuators.to_vec();

        if strict && version.compound_assignments() {
            punctuators.push(punct::COMPOUND_ASSIGNMENTS);
        }

        Self {
            dialect,
            version,
            lexicon: Lexicon::new(rules.lexer, features, rules.keywords, &punctuators),
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
            features,
        }
    }

    pub(crate) fn accepts_default(&self, nodes: &NodeStore, default: NodeId) -> bool {
        let node = nodes.get(default);

        match (&node.kind, self.parameter_defaults) {
            (_, ParameterDefaultPolicy::Expression) | (NodeKind::Literal(_), _) => true,
            (NodeKind::Parenthesis(parenthesis), ParameterDefaultPolicy::ParenthesizedLiterals) => {
                self.accepts_default(nodes, parenthesis.expression)
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
    pub fn allows(&self, features: SyntaxFeatures) -> bool {
        self.features.contains(features)
    }
}

#[cfg(test)]
#[path = "tests/grammar.rs"]
mod tests;
