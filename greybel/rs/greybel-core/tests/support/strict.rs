use greybel_core::{Dialect, ParserOptions, Version, parse};

/// Greybel's syntax, which Grey Hack's compiler rejects.
pub const GREYBEL_SYNTAX: [&str; 13] = [
    "x = 5 & 3",
    "x = 5 | 3",
    "x = 1 << 2",
    "x = 8 >> 1",
    "x = 8 >>> 1",
    "x = 1\nx += 1",
    "/* block */\nx = 1",
    "x = #envar HOME",
    "x = #line",
    "x = #filename",
    "#include \"/lib.src\"",
    "f = function(a)\nend function\nf(1,\n)",
    "f = function(a)\nend function\nf(1,)",
];

/// Whether `source` parses in `dialect` at `version`, in strict mode or not.
pub fn parses(source: &str, dialect: Dialect, version: Version, strict_mode: bool) -> bool {
    let options = ParserOptions {
        version,
        strict_mode,
        ..ParserOptions::new(dialect)
    };

    parse(source, options).is_ok()
}
