use asset_bundle::{Bundle, Fingerprint};

#[test]
fn should_hold_the_files_its_fingerprint_lists() {
    let committed = include_str!("../fingerprint.json");
    let fingerprint = Fingerprint::read(committed).expect("fingerprint.json is a fingerprint");
    let bundle =
        Bundle::read(include_bytes!("../assets.bundle")).expect("assets.bundle is a bundle");

    let actual = bundle.fingerprint(&fingerprint.game);

    assert_eq!(
        actual.to_json(),
        committed,
        "run `make assets` in greybel/dotnet"
    );
}
