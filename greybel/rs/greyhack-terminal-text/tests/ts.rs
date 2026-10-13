//! Generates terminal text protocol TypeScript in `greybel/ts/greyhack-terminal-text`. Set `UPDATE_TS=1` when running
//! `cargo test -p greyhack-terminal-text` to regenerate it.

use duplex_ts::{Module, Role};
use greybel_test_support::protocols::{PROTOCOL, major};

use crate::support::ts::terminal_text;

#[test]
fn should_generate_the_protocol() {
    terminal_text().assert_up_to_date(
        "src/protocol.generated.ts",
        Module::protocol(major("terminal-text", 1)),
    );
}

#[test]
fn should_generate_the_terminal() {
    terminal_text().assert_up_to_date(
        "src/terminal.generated.ts",
        Module::role(major("terminal-text", 1), Role::Client).with_protocol(PROTOCOL),
    );
}
