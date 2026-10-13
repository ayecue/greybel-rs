//! Generates TypeScript modules for the I/O protocol. Set `UPDATE_TS=1` when running
//! `cargo test -p greybel-io-web` to regenerate them.

use duplex_ts::{Module, Role};
use greybel_test_support::protocols::{PROTOCOL, major, typescript};

#[test]
fn should_generate_the_protocol() {
    let major = major("io", 1);

    typescript("greybel-io-web")
        .assert_up_to_date("src/protocol.generated.ts", Module::protocol(major));
}

#[test]
fn should_generate_the_page() {
    let major = major("io", 1);

    typescript("greybel-io-web").assert_up_to_date(
        "src/page.generated.ts",
        Module::role(major, Role::Provider).with_protocol(PROTOCOL),
    );
}

#[test]
fn should_generate_the_program() {
    let major = major("io", 1);

    typescript("greybel-io-web").assert_up_to_date(
        "src/program.generated.ts",
        Module::role(major, Role::Client).with_protocol(PROTOCOL),
    );
}
