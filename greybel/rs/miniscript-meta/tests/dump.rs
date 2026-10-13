//! Snapshots of everything observable about the bundled container (`greybel_test_support::meta::snapshot_parts`).
//! They were first recorded matching the TypeScript package's dump, apart from the differences PORTING.md lists.

use greybel_test_support::meta::snapshot_parts;

#[test]
fn should_dump_the_container() {
    let parts = snapshot_parts(miniscript_meta::container());

    for (name, part) in parts {
        insta::assert_yaml_snapshot!(name, part);
    }
}
