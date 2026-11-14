//! Verifies the generated world-protocol TypeScript module. Rewrite with `UPDATE_TS=1 cargo test -p
//! greyhack-world-router`.

use duplex_ts::Module;
use greybel_test_support::protocols::{major, typescript};

#[test]
fn should_generate_the_protocol() {
    typescript("greyhack-world-operations").assert_up_to_date(
        "src/protocol.generated.ts",
        Module::protocol(major("world", 1)),
    );
}
