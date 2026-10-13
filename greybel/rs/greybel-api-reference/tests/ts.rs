//! Generates TypeScript modules for the API reference and workbench protocols in
//! `greybel/ts/greybel-api-reference-web`. Set `UPDATE_TS=1` when running `cargo test -p greybel-api-reference` to
//! regenerate them.

use duplex_ts::{Module, Role};
use greybel_test_support::protocols::{PROTOCOL, major};

use crate::support::ts::{WORKBENCH, api_reference_web};

#[test]
fn should_generate_the_protocol() {
    api_reference_web().assert_up_to_date(
        "src/protocol.generated.ts",
        Module::protocol(major("api-reference", 1)),
    );
}

#[test]
fn should_generate_the_reader() {
    api_reference_web().assert_up_to_date(
        "src/reader.generated.ts",
        Module::role(major("api-reference", 1), Role::Client).with_protocol(PROTOCOL),
    );
}

#[test]
fn should_generate_the_workbench_protocol() {
    api_reference_web().assert_up_to_date(
        "src/workbench.generated.ts",
        Module::protocol(major("workbench", 1)),
    );
}

#[test]
fn should_generate_the_workbench_page() {
    api_reference_web().assert_up_to_date(
        "src/workbench-page.generated.ts",
        Module::role(major("workbench", 1), Role::Client).with_protocol(WORKBENCH),
    );
}

#[test]
fn should_generate_the_workbench_editor() {
    api_reference_web().assert_up_to_date(
        "src/workbench-editor.generated.ts",
        Module::role(major("workbench", 1), Role::Provider).with_protocol(WORKBENCH),
    );
}
