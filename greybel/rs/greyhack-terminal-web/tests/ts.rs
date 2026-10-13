//! Generates terminal protocol TypeScript modules. Set `UPDATE_TS=1` when running
//! `cargo test -p greyhack-terminal-web` to regenerate them.

use duplex_ts::{Module, Role};
use greybel_test_support::protocols::{PROTOCOL, major};

use crate::support::ts::terminal_web;

#[test]
fn should_generate_the_protocol() {
    terminal_web().assert_up_to_date(
        "src/protocol.generated.ts",
        Module::protocol(major("terminal", 1)),
    );
}

#[test]
fn should_generate_the_page() {
    terminal_web().assert_up_to_date(
        "src/page.generated.ts",
        Module::role(major("terminal", 1), Role::Client).with_protocol(PROTOCOL),
    );
}
