//! Groups editor breakpoints by file and evaluates conditions, hit counts, and logpoints. Mock and game programs
//! stop at breakpoint lines before [`RuntimeState::breakpoint_action`](crate::runtime::RuntimeState::breakpoint_action)
//! selects pause or continue.

mod decision;
mod hit_condition;

use std::collections::BTreeMap;

pub(crate) use decision::BreakpointAction;
use hit_condition::HitCondition;

use crate::protocol::{Breakpoint, SourceBreakpoint};

/// Breakpoint rules indexed by document URI.
#[derive(Debug, Default)]
pub(crate) struct BreakpointTable {
    files: BTreeMap<String, Vec<BreakpointRule>>,
}

/// Runtime breakpoint state: condition, hit predicate, log message, and hit count.
#[derive(Debug, Clone)]
struct BreakpointRule {
    line: u32,
    condition: Option<String>,
    hit_condition: Option<HitCondition>,
    log_message: Option<String>,
    /// Hits with a true condition since launch.
    hits: u32,
}

impl BreakpointTable {
    /// Replaces a file's breakpoints and returns verification results in input order. Invalid hit conditions produce
    /// unverified breakpoints and are excluded from the rules.
    pub(crate) fn set(&mut self, uri: String, breakpoints: &[SourceBreakpoint]) -> Vec<Breakpoint> {
        let mut rules = Vec::new();
        let set = breakpoints
            .iter()
            .map(|breakpoint| {
                let hit_condition = breakpoint.hit_condition.as_deref().map(HitCondition::parse);

                if let Some(Err(message)) = hit_condition {
                    return Breakpoint {
                        line: breakpoint.line,
                        verified: false,
                        message: Some(message.to_string()),
                    };
                }

                rules.push(BreakpointRule {
                    line: breakpoint.line,
                    condition: breakpoint
                        .condition
                        .clone()
                        .filter(|condition| !condition.trim().is_empty()),
                    hit_condition: hit_condition.and_then(Result::ok),
                    log_message: breakpoint
                        .log_message
                        .clone()
                        .filter(|message| !message.is_empty()),
                    hits: 0,
                });

                Breakpoint {
                    line: breakpoint.line,
                    verified: true,
                    message: None,
                }
            })
            .collect();

        if rules.is_empty() {
            self.files.remove(&uri);
        } else {
            self.files.insert(uri, rules);
        }

        set
    }

    /// Returns `uri`'s breakpoint lines, or an empty list.
    pub(crate) fn lines(&self, uri: &str) -> Vec<u32> {
        self.files
            .get(uri)
            .map(|rules| rules.iter().map(|rule| rule.line).collect())
            .unwrap_or_default()
    }

    /// Iterates over all breakpoint `(uri, line)` pairs.
    #[cfg(feature = "game")]
    pub(crate) fn all_lines(&self) -> impl Iterator<Item = (&str, u32)> {
        self.files
            .iter()
            .flat_map(|(uri, rules)| rules.iter().map(move |rule| (uri.as_str(), rule.line)))
    }

    /// Clears all hit counters at program launch.
    pub(crate) fn reset_hits(&mut self) {
        for rule in self.files.values_mut().flatten() {
            rule.hits = 0;
        }
    }

    fn rule(&self, uri: &str, line: u32) -> Option<&BreakpointRule> {
        self.files.get(uri)?.iter().find(|rule| rule.line == line)
    }

    fn count_hit(&mut self, uri: &str, line: u32) -> u32 {
        let Some(rule) = self
            .files
            .get_mut(uri)
            .and_then(|rules| rules.iter_mut().find(|rule| rule.line == line))
        else {
            return 0;
        };

        rule.hits += 1;

        rule.hits
    }
}
