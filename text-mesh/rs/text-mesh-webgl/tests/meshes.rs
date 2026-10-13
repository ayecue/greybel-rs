//! Compares Unity-recorded meshes, excluding decoration quads omitted by `vertexCount`.

use text_mesh_test_support::reference::assert_known;

use crate::support::{KNOWN_DIFFERENCES, reference};

#[test]
fn should_build_the_meshes_text_mesh_pro_builds_for_every_case() {
    let reference = reference();
    let captures = reference.captures();

    let differences = reference.differences(&captures);

    assert_known(&differences, KNOWN_DIFFERENCES);
}

#[test]
fn should_capture_over_the_setups_assets() {
    let reference = reference();
    let recorded = reference.recorded_asset_fingerprints();

    let fingerprints = reference.asset_fingerprints(recorded.keys());

    assert_eq!(
        fingerprints, recorded,
        "the meshes were captures over other font or sprite assets than the test setup's: refresh them with REFRESH_REFERENCE=1"
    );
}
