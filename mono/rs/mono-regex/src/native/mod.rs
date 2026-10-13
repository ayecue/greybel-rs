//! Executes .NET patterns with `fancy-regex`, using a step limit to implement the match timeout.

mod ast;
mod class;
mod cursor;
mod emit;
mod error;
mod groups;
mod options;
mod parser;
mod pattern;
mod replacement;

use std::borrow::Cow;

pub use error::PatternError;
use fancy_regex::{Captures, Regex, RegexBuilder};
use groups::GroupTable;
use replacement::Replacement;

use super::{Match, RegexEngine, RegexError, RegexOptions};

const DEFAULT_BACKTRACK_LIMIT: usize = 1_000_000;

#[derive(Debug, Clone, Copy)]
pub struct NativeRegexEngine {
    backtrack_limit: usize,
}

impl NativeRegexEngine {
    #[must_use]
    pub const fn new(backtrack_limit: usize) -> Self {
        Self { backtrack_limit }
    }
}

impl Default for NativeRegexEngine {
    fn default() -> Self {
        Self::new(DEFAULT_BACKTRACK_LIMIT)
    }
}

impl RegexEngine for NativeRegexEngine {
    fn is_match(
        &self,
        input: &str,
        pattern: &str,
        options: RegexOptions,
    ) -> Result<bool, RegexError> {
        CompiledRegex::new(pattern, options, self.backtrack_limit)?.is_match(input)
    }

    fn matches(
        &self,
        input: &str,
        pattern: &str,
        options: RegexOptions,
    ) -> Result<Vec<Match>, RegexError> {
        CompiledRegex::new(pattern, options, self.backtrack_limit)?.matches(input)
    }

    fn split(
        &self,
        input: &str,
        pattern: &str,
        options: RegexOptions,
    ) -> Result<Vec<String>, RegexError> {
        CompiledRegex::new(pattern, options, self.backtrack_limit)?.split(input)
    }

    fn replace(
        &self,
        input: &str,
        pattern: &str,
        replacement: &str,
        options: RegexOptions,
    ) -> Result<String, RegexError> {
        CompiledRegex::new(pattern, options, self.backtrack_limit)?.replace(input, replacement)
    }
}

fn to_failure(error: &impl ToString) -> RegexError {
    RegexError::Failure(error.to_string())
}

fn match_error(error: &fancy_regex::Error) -> RegexError {
    match error {
        fancy_regex::Error::RuntimeError(_) => RegexError::Timeout,
        error => to_failure(error),
    }
}

struct CompiledRegex {
    regex: Regex,
    groups: GroupTable,
}

impl CompiledRegex {
    fn new(
        pattern: &str,
        options: RegexOptions,
        backtrack_limit: usize,
    ) -> Result<Self, RegexError> {
        let pattern::Pattern { source, groups } =
            pattern::translate(pattern, options).map_err(|error| to_failure(&error))?;
        let regex = RegexBuilder::new(&source)
            .backtrack_limit(backtrack_limit)
            .build()
            .map_err(|error| to_failure(&error))?;

        Ok(Self { regex, groups })
    }

    fn is_match(&self, input: &str) -> Result<bool, RegexError> {
        self.regex
            .is_match(input)
            .map_err(|error| match_error(&error))
    }

    fn matches(&self, input: &str) -> Result<Vec<Match>, RegexError> {
        let mut index = 0;
        let mut last_start = 0;

        self.regex
            .find_iter(input)
            .map(|regex_match| {
                let regex_match = regex_match.map_err(|error| match_error(&error))?;

                index += input[last_start..regex_match.start()]
                    .encode_utf16()
                    .count();
                last_start = regex_match.start();

                Ok(Match {
                    index,
                    value: regex_match.as_str().to_owned(),
                })
            })
            .collect()
    }

    fn split(&self, input: &str) -> Result<Vec<String>, RegexError> {
        let mut parts = Vec::new();
        let mut previous = 0;

        for captures in self.regex.captures_iter(input) {
            let captures = captures.map_err(|error| match_error(&error))?;
            let full_match = captures.get(0).expect("group 0 is the match");

            parts.push(input[previous..full_match.start()].to_owned());
            parts.extend(
                self.groups
                    .indices()
                    .filter_map(|index| captures.get(index))
                    .map(|group| group.as_str().to_owned()),
            );
            previous = full_match.end();
        }

        parts.push(input[previous..].to_owned());

        Ok(parts)
    }

    fn replace(&self, input: &str, replacement: &str) -> Result<String, RegexError> {
        let replacement = Replacement::new(replacement, &self.groups, input)
            .map_err(|error| to_failure(&error))?;
        let output = match replacement.template() {
            Some(template) => self.regex.try_replacen(input, 0, template),
            None => self
                .regex
                .try_replacen(input, 0, |captures: &Captures<'_>| {
                    replacement.expand(captures, input)
                }),
        };

        output
            .map(Cow::into_owned)
            .map_err(|error| match_error(&error))
    }
}
