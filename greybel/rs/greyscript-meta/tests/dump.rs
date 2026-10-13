//! Snapshots the bundled container; differences are listed in `PORTING.md`.

use greybel_test_support::meta::snapshot_parts;

#[test]
fn should_dump_the_container() {
    let parts = snapshot_parts(greyscript_meta::container());

    for (name, part) in parts {
        insta::assert_yaml_snapshot!(name, part);
    }
}
