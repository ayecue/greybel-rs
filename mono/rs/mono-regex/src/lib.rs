#[cfg(feature = "dotnet")]
mod dotnet;
mod native;

#[cfg(feature = "dotnet")]
pub use dotnet::DotnetRegexEngine;
pub use native::{NativeRegexEngine, PatternError};

/// Engine selected when the `dotnet` feature is disabled.
#[cfg(feature = "dotnet")]
pub type DefaultRegexEngine = DotnetRegexEngine;
/// Engine selected when the `dotnet` feature is disabled.
#[cfg(not(feature = "dotnet"))]
pub type DefaultRegexEngine = NativeRegexEngine;

bitflags::bitflags! {
    /// .NET `RegexOptions` flags supported by both engines.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub struct RegexOptions: i32 {
        const IGNORE_CASE = 1;
        const MULTILINE = 2;
        const EXPLICIT_CAPTURE = 4;
        const SINGLELINE = 16;
        const IGNORE_PATTERN_WHITESPACE = 32;
    }
}

/// One result from `Regex.Matches`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Match {
    /// Match start offset in UTF-16 code units, matching .NET's `Match.Index`.
    pub index: usize,
    pub value: String,
}

/// Regex call failure.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RegexError {
    /// Corresponds to .NET's `RegexMatchTimeoutException`.
    #[error("the match timed out")]
    Timeout,
    /// Any other exception and its message, for example
    /// `parsing "(" - Not enough )'s.`.
    #[error("{0}")]
    Failure(String),
}

/// Static .NET `Regex` methods using the engine's match timeout.
pub trait RegexEngine {
    /// Returns whether the pattern matches, or an error for an invalid pattern or match timeout.
    fn is_match(
        &self,
        input: &str,
        pattern: &str,
        options: RegexOptions,
    ) -> Result<bool, RegexError>;

    /// Returns all matches in order, or an error for an invalid pattern or match timeout.
    fn matches(
        &self,
        input: &str,
        pattern: &str,
        options: RegexOptions,
    ) -> Result<Vec<Match>, RegexError>;

    /// Returns text between matches followed by each match's captured groups, or an error for an invalid pattern or match timeout.
    fn split(
        &self,
        input: &str,
        pattern: &str,
        options: RegexOptions,
    ) -> Result<Vec<String>, RegexError>;

    /// Implements `Regex.Replace` with .NET `$` substitutions.
    fn replace(
        &self,
        input: &str,
        pattern: &str,
        replacement: &str,
        options: RegexOptions,
    ) -> Result<String, RegexError>;
}
