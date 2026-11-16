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

#[test]
fn should_allow_every_syntax_feature_outside_strict_mode_only() {
    let features = [
        SyntaxFeatures::NUMBER_LITERALS,
        SyntaxFeatures::TEMPLATES,
        SyntaxFeatures::DECLARATIONS,
        SyntaxFeatures::ENUM,
        SyntaxFeatures::SPREAD,
        SyntaxFeatures::MAP_SHORTHAND,
        SyntaxFeatures::ASSERT,
        SyntaxFeatures::SHORT_FUNCTIONS,
        SyntaxFeatures::DESTRUCTURING_ASSIGNMENTS,
        SyntaxFeatures::FOR_INDEX,
        SyntaxFeatures::SWITCH,
        SyntaxFeatures::EXPORT,
        SyntaxFeatures::TERNARY,
        SyntaxFeatures::COALESCE,
        SyntaxFeatures::OPTIONAL_CHAINS,
        SyntaxFeatures::CLASSES,
        SyntaxFeatures::TYPES,
    ];
    let grammar = |dialect, strict| Grammar::get(dialect, Version::Latest, strict);

    let allowed = features.map(|feature| {
        [
            grammar(Dialect::Greybel, false).allows(feature),
            grammar(Dialect::GreyScript, false).allows(feature),
            grammar(Dialect::Greybel, true).allows(feature),
            grammar(Dialect::GreyScript, true).allows(feature),
            grammar(Dialect::MiniScript, false).allows(feature),
        ]
    });

    assert_eq!(allowed, [[true, true, false, false, false]; 17]);
    assert_eq!(
        features
            .into_iter()
            .fold(SyntaxFeatures::NONE, |all, feature| all | feature),
        SyntaxFeatures::ALL
    );
}

#[test]
fn should_build_a_grammar_without_the_features_turned_off() {
    let features = SyntaxFeatures::ALL.without(SyntaxFeatures::TERNARY);

    let grammar = Grammar::with_features(Dialect::Greybel, Version::Latest, false, features);
    let again = Grammar::with_features(Dialect::Greybel, Version::Latest, false, features);
    let full = Grammar::with_features(
        Dialect::Greybel,
        Version::Latest,
        false,
        SyntaxFeatures::ALL,
    );

    assert!(!grammar.allows(SyntaxFeatures::TERNARY));
    assert!(grammar.allows(SyntaxFeatures::COALESCE));
    assert!(std::ptr::eq(grammar, again));
    assert!(std::ptr::eq(
        full,
        Grammar::get(Dialect::Greybel, Version::Latest, false)
    ));
}

#[test]
fn should_read_features_by_their_names() {
    let features = SyntaxFeatures::except(["ternary", "classes", "unknown"]);

    assert_eq!(
        SyntaxFeatures::by_name("optional-chains"),
        Some(SyntaxFeatures::OPTIONAL_CHAINS)
    );
    assert!(!features.contains(SyntaxFeatures::TERNARY));
    assert!(!features.contains(SyntaxFeatures::CLASSES));
    assert!(features.contains(SyntaxFeatures::TYPES));
}
