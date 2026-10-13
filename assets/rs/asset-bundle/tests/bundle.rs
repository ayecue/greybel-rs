use asset_bundle::{Bundle, BundleError, BundleWriter};
use support::bundle::{contents, fixture, leaked, written};

use crate::support;

#[test]
fn should_read_back_every_file_written() {
    let bundle = Bundle::read(written()).expect("a bundle");

    let files = (
        bundle.text("texts/en.json"),
        bundle.text("names/empty.txt"),
        bundle.file("atlas.raw").map(<[u8]>::len),
    );

    assert_eq!(files, (Some("{\"a\": 1}"), Some(""), Some(4096)));
}

#[test]
fn should_list_the_files_in_the_order_written() {
    let bundle = Bundle::read(written()).expect("a bundle");

    let names: Vec<&str> = bundle.names().collect();

    assert_eq!(names, ["texts/en.json", "names/empty.txt", "atlas.raw"]);
}

#[test]
fn should_compress_repeated_contents() {
    let contents = vec![7u8; 65536];

    let bundle = BundleWriter::new()
        .with_file("atlas.raw", contents)
        .finish();

    assert!(bundle.len() < 1024, "{} bytes", bundle.len());
}

#[test]
fn should_find_no_file_by_an_unknown_name() {
    let bundle = Bundle::read(written()).expect("a bundle");

    let file = bundle.file("missing");

    assert!(file.is_none());
}

#[test]
fn should_refuse_bytes_without_the_magic() {
    let bytes = leaked(b"PNG\0\0\0\0\0\0".to_vec());

    let error = Bundle::read(bytes).expect_err("not a bundle");

    assert!(matches!(error, BundleError::Magic));
}

#[test]
fn should_refuse_a_bundle_cut_short() {
    let full = written();
    let cut = &full[..full.len() - 1];

    let error = Bundle::read(cut).expect_err("cut short");

    assert!(matches!(error, BundleError::Incomplete("contents")));
}

#[test]
fn should_unpack_a_file_for_the_caller() {
    let bundle = Bundle::read(written()).expect("a bundle");

    let text = bundle.unpack_text("texts/en.json");

    assert_eq!(text.as_deref(), Some("{\"a\": 1}"));
}

#[test]
fn should_unpack_a_file_already_read() {
    let bundle = Bundle::read(written()).expect("a bundle");
    let read = bundle.file("atlas.raw").map(<[u8]>::to_vec);

    let copy = bundle.unpack("atlas.raw");

    assert_eq!(copy, read);
}

#[test]
fn should_read_a_bundle_the_dotnet_writer_wrote() {
    let bundle = Bundle::read(fixture("dotnet.bundle")).expect("a bundle");

    let files = contents(&bundle);

    assert_eq!(
        files,
        (Some("{\"a\": 1}"), Some(""), Some(&[7u8; 4096][..]))
    );
}

#[test]
fn should_read_the_bundle_the_writer_wrote_before() {
    let bundle = Bundle::read(fixture("rust.bundle")).expect("a bundle");

    let files = contents(&bundle);

    assert_eq!(
        files,
        (Some("{\"a\": 1}"), Some(""), Some(&[7u8; 4096][..]))
    );
}

#[cfg(feature = "fingerprint")]
#[test]
fn should_fingerprint_like_the_dotnet_writer() {
    let bundle = Bundle::read(fixture("dotnet.bundle")).expect("a bundle");
    let expected = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/fingerprint.json"),
    )
    .expect("the fingerprint fixture");

    let fingerprint = bundle.fingerprint("1.0").to_json();

    assert_eq!(fingerprint, expected);
}

#[cfg(feature = "fingerprint")]
#[test]
fn should_fingerprint_the_unpacked_contents() {
    let bundle =
        Bundle::read(leaked(BundleWriter::new().with_file("a", "abc").finish())).expect("a bundle");

    let fingerprint = bundle.fingerprint("1.0");

    assert_eq!(
        fingerprint.files["a"],
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}
