//! Generates TypeScript runtime protocol modules. Set `UPDATE_TS=1` when running
//! `cargo test -p greybel-runtime` to regenerate them.

use duplex_ts::{Module, Role};
use greybel_test_support::protocols::{PROTOCOL, major, typescript};

#[test]
fn should_generate_the_protocol() {
    let major = major("runtime", 1);

    typescript("greybel-runtime")
        .assert_up_to_date("src/protocol.generated.ts", Module::protocol(major));
}

#[test]
fn should_generate_the_editor() {
    let major = major("runtime", 1);

    typescript("greybel-runtime").assert_up_to_date(
        "src/editor.generated.ts",
        Module::role(major, Role::Client).with_protocol(PROTOCOL),
    );
}
