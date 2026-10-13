//! Helpers for TypeScript modules generated from the package definitions.

use duplex_test_support::generated::{GeneratedFolder, MinorVersionTable};
use duplex_ts::{Module, Role};

pub const PACKAGE_CORE: &str = "./binary/index.js";
/// Import path for core types in the package's generated modules.
pub const PACKAGE_INDEX: &str = "./index.js";
/// Import path for the core message in the package's generated modules.
pub const PACKAGE_MESSAGE: &str = "./protocol/message.js";

pub const CORE: &str = "../src/index.js";

pub const CALCULATOR_PROTOCOL: &str = "./calculator.generated.js";

/// Returns the package's generated-module folder at `folder`.
pub fn typescript(folder: &str) -> GeneratedFolder {
    GeneratedFolder::typescript_package("duplex-core", folder)
}

/// Asserts that `<name>.generated.ts` is up to date for `major` and that each
/// `<name>.<file>.generated.ts` role module imports it.
pub fn assert_protocol_modules(name: &str, major: MinorVersionTable, roles: [(Role, &str); 2]) {
    let files = typescript("test");
    let protocol = format!("./{name}.generated.js");

    files.assert_up_to_date(
        format!("{name}.generated.ts"),
        Module::protocol(major).with_core(CORE),
    );
    for (role, file) in roles {
        files.assert_up_to_date(
            format!("{name}.{file}.generated.ts"),
            Module::role(major, role)
                .with_core(CORE)
                .with_protocol(&protocol),
        );
    }
}
