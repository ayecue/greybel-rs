//! The API miniscript-textmate's grammar highlights.

use greybel_test_support::textmate::assert_grammar_matches_the_api;

#[test]
fn should_highlight_the_api_in_the_textmate_grammar() {
    assert_grammar_matches_the_api(
        miniscript_meta::container(),
        "miniscript-meta",
        "miniscript-textmate",
        "miniscript.tmLanguage.json",
    );
}
