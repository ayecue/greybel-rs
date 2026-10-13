use super::*;

#[test]
fn should_build_a_grammar_once_per_dialect_and_version() {
    let latest = Grammar::get(Dialect::MiniScript, Version::Latest, false);

    let again = Grammar::get(Dialect::MiniScript, Version::Latest, false);

    assert!(std::ptr::eq(latest, again));
}

#[test]
fn should_build_another_grammar_for_another_version() {
    let latest = Grammar::get(Dialect::MiniScript, Version::Latest, false);

    let v1_6 = Grammar::get(Dialect::MiniScript, Version::V1_6, false);

    assert!(!std::ptr::eq(latest, v1_6));
}

#[test]
fn should_build_the_greyscript_grammar_with_multiline_strings() {
    let (dialect, version) = (Dialect::GreyScript, Version::V1_6);

    let greyscript = Grammar::get(dialect, version, false);

    assert_eq!(
        (greyscript.dialect(), greyscript.version()),
        (Dialect::GreyScript, Version::V1_6)
    );
    assert!(greyscript.multiline_strings);
}

/// Grey Hack ends a string at a line break since 0.9.7373, though its other rules are MiniScript 1.6's.
#[test]
fn should_forbid_multiline_strings_in_strict_greyscript() {
    let greyscript = Grammar::get(Dialect::GreyScript, Version::V1_6, true);

    assert!(!greyscript.multiline_strings);
}

#[test]
fn should_allow_multiline_strings_in_miniscript_1_6_only() {
    let versions = [Version::Latest, Version::V1_6];

    let [latest, v1_6] = versions.map(|version| Grammar::get(Dialect::MiniScript, version, false));

    assert!(!latest.multiline_strings);
    assert!(v1_6.multiline_strings);
}
