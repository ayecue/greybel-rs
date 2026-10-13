//! The language server's settings: the `greybel` section greybel-vs sends it, with the defaults it declares.

use greybel_lsp::Settings;
use serde::Deserialize;

use crate::support::settings::section;

#[test]
fn should_give_the_language_server_its_defaults() {
    let declared = Settings::deserialize(&section()).expect("settings");

    assert_eq!(Settings::default(), declared);
}
