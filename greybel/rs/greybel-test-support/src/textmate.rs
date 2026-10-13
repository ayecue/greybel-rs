//! Generates TextMate API-name rules from a metadata container. Only `general-functions` and `type-methods` match
//! patterns are generated; the rest of the grammar is handwritten.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use duplex_test_support::generated::GeneratedFolder;
use meta_utils::Container;
use serde_json::Value;

/// Metadata type for receiverless global functions.
const GENERAL: &str = "general";

/// Checks generated API rules in grammar `file` under `greybel/ts/<textmate>/src` against `container`, owned by
/// crate `package`. Setting `UPDATE_TS=1` rewrites both rules.
///
/// # Panics
/// Panics on grammar read errors or generated-rule mismatches.
pub fn assert_grammar_matches_the_api(
    container: &Container,
    package: &'static str,
    textmate: &str,
    file: &str,
) {
    let folder = Path::new(env!("GREYBEL_TS")).join(textmate).join("src");
    let grammar = fs::read_to_string(folder.join(file)).expect("the grammar reads");

    GeneratedFolder::typescript(folder, package)
        .assert_up_to_date(file, grammar_with_api(&grammar, container));
}

/// Updates `grammar` with sorted global-function and dot-prefixed type-method alternatives generated from
/// `container`.
fn grammar_with_api(grammar: &str, container: &Container) -> String {
    let mut general = BTreeSet::new();
    let mut methods = BTreeSet::new();

    for signature in container.signatures() {
        let names = if signature.type_name() == GENERAL {
            &mut general
        } else {
            &mut methods
        };

        names.extend(
            signature
                .definitions()
                .iter()
                .filter(|(_, definition)| definition.as_function().is_some())
                .map(|(name, _)| name.as_str()),
        );
    }

    let grammar = replace_match(grammar, "general-functions", &alternatives(r"\b", &general));

    replace_match(&grammar, "type-methods", &alternatives(r"\.", &methods))
}

fn alternatives(prefix: &str, names: &BTreeSet<&str>) -> String {
    format!(
        r"{prefix}({})\b",
        names.iter().copied().collect::<Vec<_>>().join("|")
    )
}

/// Replaces one serialized `repository[rule].match` string with `pattern` and preserves all other grammar text.
fn replace_match(grammar: &str, rule: &str, pattern: &str) -> String {
    let json: Value = serde_json::from_str(grammar).expect("the grammar is JSON");
    let current = json["repository"][rule]["match"]
        .as_str()
        .unwrap_or_else(|| panic!("the grammar's rule {rule} has a match"));
    let quote = |text: &str| serde_json::to_string(text).expect("a string serializes");
    let (current, replacement) = (quote(current), quote(pattern));

    assert_eq!(
        grammar.matches(&current).count(),
        1,
        "the match of {rule} is written once"
    );

    grammar.replacen(&current, &replacement, 1)
}
