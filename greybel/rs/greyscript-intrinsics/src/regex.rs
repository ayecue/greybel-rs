//! Regex support for string intrinsics, including Grey Hack's match timeout.
//! Hosts can override the engine through [`HostRegexEngine`].

use std::rc::Rc;
#[cfg(feature = "dotnet-regex")]
use std::time::Duration;

use greybel_interpreter::{FromVm, Vm};
#[cfg(feature = "dotnet-regex")]
pub use mono_regex::DotnetRegexEngine;
pub use mono_regex::{
    DefaultRegexEngine, Match, NativeRegexEngine, RegexEngine, RegexError, RegexOptions,
};

/// Grey Hack's `matchTimeout` value (`BasicIntrinsics`).
#[cfg(feature = "dotnet-regex")]
const MATCH_TIMEOUT: Duration = Duration::from_millis(500);
/// Backtracking-step limit used in place of [`MATCH_TIMEOUT`] by `fancy-regex`.
#[cfg(not(feature = "dotnet-regex"))]
const BACKTRACK_LIMIT: usize = 1_000_000;

/// Regex engine enforcing Grey Hack's match timeout.
#[cfg(feature = "dotnet-regex")]
#[must_use]
pub fn grey_hack_engine() -> DefaultRegexEngine {
    DotnetRegexEngine::new(Some(MATCH_TIMEOUT))
}

/// Regex engine enforcing Grey Hack's match timeout.
#[cfg(not(feature = "dotnet-regex"))]
#[must_use]
pub fn grey_hack_engine() -> DefaultRegexEngine {
    NativeRegexEngine::new(BACKTRACK_LIMIT)
}

/// Parses `regexOptions`: `none` or one .NET option letter (Grey Hack's `ToRegexOptions`).
#[must_use]
pub fn parse_options(option: &str) -> Option<RegexOptions> {
    Some(match option {
        "none" => RegexOptions::empty(),
        "i" => RegexOptions::IGNORE_CASE,
        "m" => RegexOptions::MULTILINE,
        "s" => RegexOptions::SINGLELINE,
        "n" => RegexOptions::EXPLICIT_CAPTURE,
        "x" => RegexOptions::IGNORE_PATTERN_WHITESPACE,
        _ => return None,
    })
}

/// Host-selected regex engine for a VM.
/// Install with `interpreter.with_host_data(HostRegexEngine::new(engine))`.
#[derive(Clone)]
pub struct HostRegexEngine(Rc<dyn RegexEngine>);

impl HostRegexEngine {
    pub fn new(engine: impl RegexEngine + 'static) -> Self {
        Self(Rc::new(engine))
    }
}

impl Default for HostRegexEngine {
    fn default() -> Self {
        Self::new(grey_hack_engine())
    }
}

impl std::ops::Deref for HostRegexEngine {
    type Target = dyn RegexEngine;

    fn deref(&self) -> &Self::Target {
        &*self.0
    }
}

impl FromVm for HostRegexEngine {
    fn from_vm(vm: &Vm) -> Self {
        vm.host_data::<Self>().cloned().unwrap_or_default()
    }
}
