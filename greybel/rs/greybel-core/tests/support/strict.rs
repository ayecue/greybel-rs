use greybel_core::{Dialect, ParserOptions, Version, parse};

/// Greybel's syntax, which Grey Hack's compiler rejects.
pub const GREYBEL_SYNTAX: [&str; 39] = [
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
    "x = 0xFF",
    "x = 0b1010",
    "x = 1_000_000",
    "let x = 1",
    "const X = 1",
    "x = #\"a{1}\"",
    "enum Color\n\tRed\nend enum",
    "x = [...a]",
    "x = {...a}",
    "x = {a}",
    "x = 0o17",
    "f = (x) => x",
    "[a, b] = c",
    "{a} = c",
    "for i, v in l\nend for",
    "for [a] in l\nend for",
    "switch x\ncase 1\nend switch",
    "export x = 1",
    "x = a ? b : c",
    "x = a ?? b",
    "x = a?.b",
    "class A\n  x = 1\nend class",
    "f = function(a: number)\nend function",
    "x = y!",
    "x = y as number",
    "type Id = number",
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
