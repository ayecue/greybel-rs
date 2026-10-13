//! Set `UPDATE_TS=1` when running `cargo test -p duplex-core` to regenerate the TypeScript files.

use std::path::Path;

use duplex_test_support::conformance::definition;
use duplex_test_support::generated::major;
use duplex_ts::{Module, Role};

use crate::support::ts::{
    CALCULATOR_PROTOCOL, CORE, PACKAGE_CORE, PACKAGE_INDEX, PACKAGE_MESSAGE,
    assert_protocol_modules, typescript,
};

#[test]
fn should_generate_the_calculator_modules_for_the_ts_tests() {
    let major = major(definition("calculator"), 1);

    assert_protocol_modules(
        "calculator",
        major,
        [(Role::Client, "client"), (Role::Provider, "provider")],
    );
}

#[test]
fn should_generate_the_calculator_2_and_its_migration_for_the_ts_tests() {
    let (older, newer) = (
        major(definition("calculator"), 1),
        major(definition("calculator"), 2),
    );

    typescript("test").assert_up_to_date(
        "calculator.v2.generated.ts",
        Module::protocol(newer).with_core(CORE),
    );
    typescript("test").assert_up_to_date(
        "calculator.v2.provider.generated.ts",
        Module::role(newer, Role::Provider)
            .with_core(CORE)
            .with_protocol("./calculator.v2.generated.js"),
    );
    typescript("test").assert_up_to_date(
        "calculator.migration.generated.ts",
        Module::migration(
            older,
            newer,
            CALCULATOR_PROTOCOL,
            "./calculator.v2.generated.js",
        )
        .with_core(CORE),
    );
}

#[test]
fn should_generate_the_login_modules_for_the_ts_tests() {
    let major = major(definition("login"), 1);

    assert_protocol_modules(
        "login",
        major,
        [(Role::Client, "member"), (Role::Provider, "service")],
    );
}

#[test]
fn should_generate_the_vault_modules_for_the_ts_tests() {
    let major = major(definition("vault"), 1);

    assert_protocol_modules(
        "vault",
        major,
        [(Role::Client, "asker"), (Role::Provider, "keeper")],
    );
}

#[test]
fn should_generate_the_legacy_server_for_the_ts_tests() {
    let major = major(definition("legacy"), 1);

    typescript("test").assert_up_to_date(
        "legacy.server.generated.ts",
        Module::role(major, Role::Provider).with_core(CORE),
    );
}

#[test]
fn should_generate_the_layout_types_for_the_ts_tests() {
    let major = major(definition("layouts"), 1);

    typescript("test")
        .assert_up_to_date("layouts.generated.ts", Module::types(major).with_core(CORE));
}

#[test]
fn should_generate_the_sensor_for_the_ts_tests() {
    let major = major(definition("tags"), 1);

    typescript("test").assert_up_to_date(
        "sensor.generated.ts",
        Module::protocol(major).with_core(CORE),
    );
}

#[test]
fn should_generate_the_packages_own_definitions() {
    let major = major(Path::new(env!("DUPLEX")).join("core"), 1);

    typescript("src").assert_up_to_date(
        "definition.generated.ts",
        Module::types(major)
            .with_core(PACKAGE_CORE)
            .with_core_types(PACKAGE_INDEX)
            .with_core_message(PACKAGE_MESSAGE),
    );
}
