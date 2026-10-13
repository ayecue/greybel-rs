//! Compares layouts with Unity recordings, except the unavailable source index.

use text_mesh_test_support::reference::assert_known;

use crate::support::{KNOWN_DIFFERENCES, captures, compare, lay_out};

#[test]
fn should_lay_out_every_case_like_text_mesh_pro() {
    let captures = captures();

    let differences = captures.differences(|differences, case, width| {
        compare(differences, &case.items, &lay_out(&case.source, width));
    });

    assert_known(&differences, KNOWN_DIFFERENCES);
}
