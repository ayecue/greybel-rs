use reman::config::Kind;
use reman::init::{InitOptions, KindPattern};

use super::Fixture;

/// A Cargo workspace `rs` without reman files but the root's: `cli` builds a program, `core` a library, `helpers`
/// is unpublished, `fixtures` a library in a folder named like test data.
pub fn workspace() -> Fixture {
    let fixture = Fixture::new();

    fixture
        .file(
            "rs/Cargo.toml",
            "[workspace]\nmembers = [\"*\"]\n\n[workspace.package]\nversion = \"0.1.0\"\n",
        )
        .crate_("rs", "core", &[], None)
        .crate_("rs", "cli", &["core"], None)
        .file("rs/cli/src/main.rs", "fn main() {}\n")
        .file(
            "rs/helpers/Cargo.toml",
            "[package]\nname = \"helpers\"\nversion = \"0.1.0\"\npublish = false\n",
        )
        .file("rs/helpers/src/lib.rs", "")
        .crate_("rs", "fixtures", &[], None);

    fixture
}

/// Options with `dev` patterns and `kinds`.
pub fn options(dev: &[&str], kinds: &[(Kind, &str)]) -> InitOptions {
    let kind_pattern = |(kind, pattern): &(Kind, &str)| KindPattern {
        kind: *kind,
        pattern: (*pattern).to_owned(),
    };

    InitOptions {
        date: jiff::civil::date(2026, 1, 1),
        kinds: kinds.iter().map(kind_pattern).collect(),
        dev: dev.iter().map(|pattern| (*pattern).to_owned()).collect(),
    }
}
