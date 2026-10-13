//! Generates editor and server TypeScript modules for the documents protocol. Set `UPDATE_TS=1` to regenerate them.

use duplex_ts::{Module, Role};
use greybel_test_support::protocols::{PROTOCOL, major, typescript};

#[test]
fn should_generate_the_protocol() {
    let major = major("documents", 1);

    typescript("greybel-documents-host-provider")
        .assert_up_to_date("src/protocol.generated.ts", Module::protocol(major));
}

#[test]
fn should_generate_the_editor() {
    let major = major("documents", 1);

    typescript("greybel-documents-host-provider").assert_up_to_date(
        "src/editor.generated.ts",
        Module::role(major, Role::Provider).with_protocol(PROTOCOL),
    );
}

#[test]
fn should_generate_the_server_of_the_ts_tests() {
    let major = major("documents", 1);

    typescript("greybel-documents-host-provider").assert_up_to_date(
        "test/server.generated.ts",
        Module::role(major, Role::Client).with_protocol("../src/protocol.generated.js"),
    );
}
