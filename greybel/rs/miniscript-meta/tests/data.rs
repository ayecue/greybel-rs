//! Checks the bundled data for mistakes no payload format catches.

use greybel_test_support::meta::{assert_no_mistakes, consistency_mistakes};

#[test]
fn should_bundle_consistent_data() {
    let container = miniscript_meta::container();

    let mistakes = consistency_mistakes(container);

    assert_no_mistakes(&mistakes);
}
