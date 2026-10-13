use mono_regex::{NativeRegexEngine, RegexEngine, RegexOptions};

pub const NONE: RegexOptions = RegexOptions::empty();

pub fn split(input: &str, pattern: &str, options: RegexOptions) -> Vec<String> {
    NativeRegexEngine::default()
        .split(input, pattern, options)
        .expect("splits")
}

pub fn is_match(input: &str, pattern: &str, options: RegexOptions) -> bool {
    NativeRegexEngine::default()
        .is_match(input, pattern, options)
        .expect("matches")
}
