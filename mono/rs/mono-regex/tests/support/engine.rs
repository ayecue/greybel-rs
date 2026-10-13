use std::time::Duration;

use mono_regex::{DotnetRegexEngine, RegexError, RegexOptions};

pub const NONE: RegexOptions = RegexOptions::empty();

/// The options as .NET names them, such as `IgnoreCase` or `None`.
pub fn dotnet_names(options: RegexOptions) -> String {
    if options.is_empty() {
        return "None".into();
    }

    let pascal = |name: &str| -> String {
        name.split('_')
            .map(|word| word[..1].to_owned() + &word[1..].to_lowercase())
            .collect()
    };

    options
        .iter_names()
        .map(|(name, _)| pascal(name))
        .collect::<Vec<_>>()
        .join(" | ")
}

/// Grey Hack's `matchTimeout`.
pub fn dotnet() -> DotnetRegexEngine {
    DotnetRegexEngine::new(Some(Duration::from_millis(500)))
}

/// Patterns run on both engines; where the translation reads them
/// differently from the game, the snapshot says how.
pub const CASES: &[(&str, &str, RegexOptions)] = &[
    ("a1b22c333", "[0-9]+", RegexOptions::empty()),
    ("a-b+c", "([-+])", RegexOptions::empty()),
    ("a-b", "(-)", RegexOptions::EXPLICIT_CAPTURE),
    ("one.two", ".", RegexOptions::empty()),
    ("one.two", r"\.", RegexOptions::empty()),
    ("John Smith", r"(\w+) (\w+)", RegexOptions::empty()),
    ("aaa", "A", RegexOptions::IGNORE_CASE),
    ("line1\nline2", "^line2$", RegexOptions::MULTILINE),
    ("x(", "(", RegexOptions::empty()),
    ("aé😀b😀", "😀|b", RegexOptions::empty()),
    ("a1b2", r"\d", RegexOptions::empty()),
    ("ab\n", "b$", RegexOptions::empty()),
    ("ab\n", r"b\Z", RegexOptions::empty()),
    ("a\nb", "a.b", RegexOptions::SINGLELINE),
    ("a{,2}", "a{,2}", RegexOptions::empty()),
    ("abcde", "[a-z-[aeiou]]+", RegexOptions::empty()),
    ("a&&b", "[&&]", RegexOptions::empty()),
    ("abab", r"(?<x>a)(b)\k<x>\1", RegexOptions::empty()),
    ("cd ab", "(?(a)ab|cd)", RegexOptions::empty()),
    ("ab c", "(?<x>a)(?(x)b|c)", RegexOptions::empty()),
    ("aBc", "a(?i)b(?-i)c", RegexOptions::empty()),
    (
        "a b c",
        "a b # comment",
        RegexOptions::IGNORE_PATTERN_WHITESPACE,
    ),
    ("Straße STRASSE", "straße", RegexOptions::IGNORE_CASE),
    ("İstanbul", "i", RegexOptions::IGNORE_CASE),
    ("héllo wörld_1", r"\w+", RegexOptions::empty()),
    ("a\u{200D}b", r"\w+", RegexOptions::empty()),
    ("x y", r"\b", RegexOptions::empty()),
    ("😀", ".", RegexOptions::empty()),
    ("😀", "", RegexOptions::empty()),
    ("ab", r"\p{Lu}|\p{Ll}", RegexOptions::empty()),
    ("ab", r"\p{IsBasicLatin}", RegexOptions::empty()),
    ("a1", r"[\w-[\d]]", RegexOptions::empty()),
    ("aaa", "a*?", RegexOptions::empty()),
    ("aaa", "(?>a+)a", RegexOptions::empty()),
    ("abc", "(?<=a)b", RegexOptions::empty()),
    ("abc", "a(?#comment)b", RegexOptions::empty()),
    ("abc", r"\x61\u0062\143", RegexOptions::empty()),
    ("a\tb", r"a\cIb", RegexOptions::empty()),
    ("abc", "(?<1>a)(b)", RegexOptions::empty()),
    ("abc", "[]a]", RegexOptions::empty()),
    ("abc", "[^]a]", RegexOptions::empty()),
    ("a-b", "[a\\-z]", RegexOptions::empty()),
    ("abc", r"\q", RegexOptions::empty()),
    ("abc", "(?<a-b>x)", RegexOptions::empty()),
    ("abc", "(?<x>a)(?<x>b)", RegexOptions::empty()),
    ("abc", "(?i:A)B", RegexOptions::empty()),
    ("abc", "a{2,1}", RegexOptions::empty()),
    ("abc", "[z-a]", RegexOptions::empty()),
    ("abc", r"(a)\2", RegexOptions::empty()),
    ("aBC", "(a(?i)b|c)C", RegexOptions::empty()),
    ("a\u{8}A", r"\10\101", RegexOptions::empty()),
    ("ab", r"(a)\1?b\k<1>?", RegexOptions::empty()),
    ("cd ab", "(?(x)ab|cd)", RegexOptions::empty()),
    ("ab cd", "(?((a))ab|cd)", RegexOptions::empty()),
    ("ab", "(?(?i)a|b)", RegexOptions::empty()),
    ("a-b", r"[a-\-]", RegexOptions::empty()),
    ("a-", r"[\--a]", RegexOptions::empty()),
    ("[a]", "[[:a:]]", RegexOptions::empty()),
    ("ab", "(?<b>a)(?<a-b>b)", RegexOptions::empty()),
    ("ab", "(?<a>a)(?<a>b)", RegexOptions::empty()),
    ("aa", "a*(?i)*", RegexOptions::empty()),
    ("aa", "a*(?#c)+", RegexOptions::empty()),
    ("aa", "a{2}{3}", RegexOptions::empty()),
    ("a b", "a\\ b", RegexOptions::IGNORE_PATTERN_WHITESPACE),
    (
        "ab",
        "a (?#x) b # end",
        RegexOptions::IGNORE_PATTERN_WHITESPACE,
    ),
    ("ab", "(?x: a b )", RegexOptions::empty()),
    ("AB", "(?i:a)B", RegexOptions::empty()),
    ("a\nb", "a(?s:.)b|(?m)^b", RegexOptions::empty()),
    ("abab", "(?<n>ab)\\k'n'", RegexOptions::empty()),
    ("ab", "(?'x'a)(?(x)b)", RegexOptions::empty()),
    ("ab", "(?(1a)b)", RegexOptions::empty()),
    ("ab", "(?(?#c)a)", RegexOptions::empty()),
    ("ab", "(?(?'x'a)b)", RegexOptions::empty()),
    ("ab", "(a)(?(1)b|c|d)", RegexOptions::empty()),
    ("ab", "(?<0>a)", RegexOptions::empty()),
    ("ab", "(?<a!>a)", RegexOptions::empty()),
    ("ab", "[a-\\d]", RegexOptions::empty()),
    ("ab", "[a-[b]c]", RegexOptions::empty()),
    ("ab", "\\k", RegexOptions::empty()),
    ("ab", "\\k<x>", RegexOptions::empty()),
    ("ab", "\\pL", RegexOptions::empty()),
    ("ab", "*[a", RegexOptions::empty()),
    ("ab", r"*\q", RegexOptions::empty()),
    ("ab", "(?(a)(?i:b))", RegexOptions::empty()),
    ("ab", "(?(1)(?i)b|)(a)", RegexOptions::empty()),
    ("abc", "(?(a)(b(?i)c)|c)", RegexOptions::empty()),
    ("xbz", "(?(a)b)", RegexOptions::empty()),
    ("ab", "(?(1)b)(a)", RegexOptions::empty()),
    ("ab", "(?<a-", RegexOptions::empty()),
    ("a{L}", r"(\p{L})", RegexOptions::empty()),
];

pub const REPLACEMENTS: &[(&str, &str, &str)] = &[
    ("xaby", "(a)(b)", "$2$1"),
    ("a", "(a)", "$10"),
    ("a", "(?<x>a)", "[${x}]"),
    ("a", "a", "${nope}"),
    ("abc", "b", "[$`|$&|$']"),
    ("abc", "b", "$$"),
    ("abc", "b", "$_"),
    ("ab", "(a)(b)", "$+"),
    ("abc", "b", "$x$"),
    ("ab", "(?<x>a)(b)", "$1$2"),
    ("ab", "(?<x>a)(b)", "${2}"),
    ("ab", "a", "${0}"),
];

pub fn describe<T: std::fmt::Debug>(result: &Result<T, RegexError>) -> String {
    format!("{result:?}")
}
