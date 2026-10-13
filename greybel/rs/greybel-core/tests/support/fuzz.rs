use std::collections::BTreeMap;
use std::panic::{self, AssertUnwindSafe};
use std::sync::{Arc, Mutex};

use greybel_core::{Dialect, Parser, ParserOptions, Version};

pub const PIECES: &[&str] = &[
    "a",
    "b",
    "x1",
    "_",
    "é",
    "日本",
    "😀",
    "1",
    "2.5",
    ".5",
    "1e",
    "1e+",
    "1e3",
    ".",
    "..",
    "\"",
    "\"\"",
    "\"s\"",
    "'",
    " ",
    "\t",
    "\n",
    "\r",
    "\r\n",
    "\n\r",
    ";",
    ",",
    ":",
    "(",
    ")",
    "[",
    "]",
    "{",
    "}",
    "+",
    "-",
    "*",
    "/",
    "%",
    "^",
    "=",
    "==",
    "!=",
    "<",
    "<=",
    ">",
    ">=",
    "+=",
    "-=",
    "@",
    "&",
    "|",
    "<<",
    ">>",
    ">>>",
    "//",
    "/*",
    "*/",
    "#",
    "#if",
    "#endif",
    "#else",
    "#import",
    "#include",
    "#envar",
    "#line",
    "#filename",
    "#inject",
    "//import",
    "//include",
    "debugger",
    "import_code",
    "if",
    "then",
    "else",
    "else if",
    "end",
    "end if",
    "while",
    "end while",
    "for",
    "in",
    "end for",
    "function",
    "end function",
    "return",
    "break",
    "continue",
    "and",
    "or",
    "not",
    "isa",
    "new",
    "null",
    "true",
    "false",
    "self",
    "repeat",
    "from",
];

pub type NamedParse = (&'static str, fn(&str));

pub const DEFAULT_INPUTS: usize = 3_000;

/// Advances a deterministic xorshift64 PRNG so fuzz failures reproduce.
pub fn next(seed: &mut u64) -> u64 {
    *seed ^= *seed << 13;
    *seed ^= *seed >> 7;
    *seed ^= *seed << 17;

    *seed
}

pub fn program(seed: &mut u64) -> String {
    let pieces = 1 + next(seed) % 24;
    let mut source = String::new();

    for _ in 0..pieces {
        source.push_str(PIECES[(next(seed) % PIECES.len() as u64) as usize]);
        if next(seed).is_multiple_of(3) {
            source.push(' ');
        }
    }

    source
}

pub fn parse_all_modes(dialect: Dialect, version: Version, source: &str) {
    for preserve in [false, true] {
        let options = ParserOptions {
            version,
            preserve,
            ..ParserOptions::new(dialect)
        };

        if let Ok(ast) = Parser::new(source, options.clone()).parse() {
            let _ = ast.to_string();
        }

        let output = Parser::new(source, options).parse_unsafe();
        let _ = output.ast.to_string();
    }
}

pub fn input_count() -> usize {
    std::env::var("FUZZ_INPUTS")
        .ok()
        .and_then(|value| value.parse().ok())
        .unwrap_or(DEFAULT_INPUTS)
}

pub fn panicking_inputs(inputs: usize, dialects: &[NamedParse]) -> Vec<String> {
    let location = Arc::new(Mutex::new(String::new()));
    let recorded = Arc::clone(&location);
    let previous_hook = panic::take_hook();

    panic::set_hook(Box::new(move |info| {
        let at = info
            .location()
            .map(|at| format!("{}:{}", at.file(), at.line()))
            .unwrap_or_default();

        *recorded.lock().unwrap() = format!("{at}: {}", info.payload_as_str().unwrap_or(""));
    }));

    let mut failures: BTreeMap<String, (&str, String)> = BTreeMap::new();
    let mut seed = 0x9e37_79b9_7f4a_7c15_u64;

    for _ in 0..inputs {
        let source = program(&mut seed);

        for &(name, parse) in dialects {
            if panic::catch_unwind(AssertUnwindSafe(|| parse(&source))).is_err() {
                let site = location.lock().unwrap().clone();
                let entry = failures.entry(site).or_insert((name, source.clone()));

                if source.len() < entry.1.len() {
                    *entry = (name, source.clone());
                }
            }
        }
    }

    panic::set_hook(previous_hook);

    failures
        .iter()
        .map(|(site, (dialect, source))| format!("{site}\n  {dialect}: {source:?}"))
        .collect()
}
