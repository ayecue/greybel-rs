use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Named hexadecimal binary layouts from `duplex/conformance/fixtures.json`.
pub struct Fixtures;

impl Fixtures {
    fn path() -> PathBuf {
        Path::new(env!("DUPLEX")).join("conformance/fixtures.json")
    }

    /// Loads every fixture by name.
    #[must_use]
    pub fn all() -> BTreeMap<String, String> {
        let text = std::fs::read_to_string(Self::path()).expect("the fixtures read");

        serde_json::from_str(&text).expect("the fixtures are a JSON object of strings")
    }

    /// Returns fixture `name` or panics if absent.
    #[must_use]
    pub fn get(name: &str) -> String {
        Self::all()
            .remove(name)
            .unwrap_or_else(|| panic!("a fixture `{name}`"))
    }

    pub fn check(name: &str, actual: &str) {
        let text = std::fs::read_to_string(Self::path()).expect("the fixtures read");
        let key = format!("\"{name}\": \"");
        let location = text.find(&key).map(|start| {
            let value = start + key.len();
            let end = value + text[value..].find('"').expect("a closed string");

            (value, end)
        });

        if std::env::var_os("UPDATE_FIXTURES").is_some() {
            let updated_text = match location {
                Some((value, end)) => format!("{}{actual}{}", &text[..value], &text[end..]),
                None => {
                    let last = text.rfind('"').expect("a fixture") + 1;

                    format!(
                        "{},\n  \"{name}\": \"{actual}\"{}",
                        &text[..last],
                        &text[last..]
                    )
                }
            };

            std::fs::write(Self::path(), updated_text).expect("the fixtures write");

            return;
        }

        let expected = location.map(|(value, end)| &text[value..end]);

        assert_eq!(Some(actual), expected, "the fixture `{name}`");
    }
}
