use super::{InfixRule, KeywordStatement, PrefixRule, RuleTableSet, Trigger};
use crate::dialect::RuleTable;
use crate::lexer::{Keyword, Punct};
use crate::parser::{Parser, Precedence, PrecedenceTable};

struct Entry<R: ?Sized + 'static> {
    rule: &'static R,
    rank: u8,
}

struct RuleMap<R: ?Sized + 'static> {
    keywords: Vec<Vec<Entry<R>>>,
    puncts: Vec<Vec<Entry<R>>>,
}

impl<R: ?Sized + 'static> RuleMap<R> {
    fn new(
        tables: RuleTableSet<R>,
        triggers: impl Fn(&'static R) -> Vec<Trigger>,
        rank: impl Fn(&'static R) -> u8,
    ) -> Self {
        let mut map = Self {
            keywords: (0..Keyword::COUNT).map(|_| Vec::new()).collect(),
            puncts: (0..Punct::COUNT).map(|_| Vec::new()).collect(),
        };

        for &rule in tables.iter().flat_map(|table| table.iter()) {
            for trigger in triggers(rule) {
                map.slot(trigger).push(Entry {
                    rule,
                    rank: rank(rule),
                });
            }
        }

        map
    }

    fn slot(&mut self, trigger: Trigger) -> &mut Vec<Entry<R>> {
        match trigger {
            Trigger::Keyword(keyword) => &mut self.keywords[keyword.index()],
            Trigger::Punct(punct) => &mut self.puncts[punct.index()],
        }
    }

    fn get(&self, trigger: Trigger) -> &[Entry<R>] {
        match trigger {
            Trigger::Keyword(keyword) => &self.keywords[keyword.index()],
            Trigger::Punct(punct) => &self.puncts[punct.index()],
        }
    }
}

pub struct RuleIndex {
    precedence: PrecedenceTable,
    statements: RuleMap<dyn KeywordStatement>,
    prefix: RuleMap<dyn PrefixRule>,
    infix: RuleMap<dyn InfixRule>,
}

impl std::fmt::Debug for RuleIndex {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RuleIndex").finish_non_exhaustive()
    }
}

impl RuleIndex {
    pub(crate) fn new(rules: &RuleTable, precedence: PrecedenceTable) -> Self {
        let rank = |level: Option<Precedence>| level.map_or(0, |level| rank_in(&precedence, level));

        Self {
            statements: RuleMap::new(
                rules.statements,
                |rule| {
                    rule.keywords()
                        .iter()
                        .copied()
                        .map(Trigger::Keyword)
                        .collect()
                },
                |_| 0,
            ),
            prefix: RuleMap::new(
                rules.prefix,
                |rule| rule.triggers().to_vec(),
                |rule| rank(rule.precedence()),
            ),
            infix: RuleMap::new(
                rules.infix,
                |rule| rule.triggers().to_vec(),
                |rule| rank(Some(rule.precedence())),
            ),
            precedence,
        }
    }
}

fn rank_in(table: &PrecedenceTable, level: Precedence) -> u8 {
    table
        .rank(level)
        .unwrap_or_else(|| panic!("precedence level `{level}` is not in the dialect's table"))
}

impl Parser<'_> {
    pub(crate) fn statement_rule(&self, keyword: Keyword) -> Option<&'static dyn KeywordStatement> {
        self.grammar
            .rules
            .statements
            .get(Trigger::Keyword(keyword))
            .first()
            .map(|entry| entry.rule)
    }

    pub(crate) fn prefix_rule(&self, min: u8) -> Option<&'static dyn PrefixRule> {
        let trigger = Trigger::of(self.token())?;

        self.grammar
            .rules
            .prefix
            .get(trigger)
            .iter()
            .find(|entry| entry.rank == 0 || entry.rank >= min)
            .map(|entry| entry.rule)
    }

    pub(crate) fn infix_rule(&self) -> Option<(&'static dyn InfixRule, u8)> {
        let trigger = Trigger::of(self.token())?;
        let entry = self.grammar.rules.infix.get(trigger).first()?;

        Some((entry.rule, entry.rank))
    }

    #[must_use]
    pub fn rank(&self, level: Precedence) -> u8 {
        rank_in(&self.grammar.rules.precedence, level)
    }
}
