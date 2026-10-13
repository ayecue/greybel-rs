use super::super::Replacement;
use crate::native::pattern::translate;
use crate::{NativeRegexEngine, RegexEngine, RegexOptions};

pub(super) fn replace(pattern: &str, input: &str, replacement: &str) -> String {
    NativeRegexEngine::default()
        .replace(input, pattern, replacement, RegexOptions::empty())
        .expect("replaces")
}

pub(super) fn template(pattern: &str, replacement: &str) -> Option<String> {
    let pattern = translate(pattern, RegexOptions::empty()).expect("pattern");

    Replacement::new(replacement, &pattern.groups, "in$put")
        .expect("translates")
        .template()
        .map(str::to_owned)
}
