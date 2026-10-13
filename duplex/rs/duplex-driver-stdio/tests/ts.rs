//! Generates the TypeScript driver's conformance protocol; set `UPDATE_TS=1` to rewrite it.

use duplex_test_support::conformance::definition;
use duplex_test_support::generated::{GeneratedFolder, major};
use duplex_ts::{Module, Role};

#[test]
fn should_generate_the_user_of_the_calculator_for_the_ts_tests() {
    let major = major(definition("calculator"), 1);
    let files = GeneratedFolder::typescript_package("duplex-driver-stdio", "test");

    let module = Module::role(major, Role::Client);

    files.assert_up_to_date("calculator.client.generated.ts", module);
}
