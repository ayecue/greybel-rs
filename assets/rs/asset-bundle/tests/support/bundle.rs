use asset_bundle::{Bundle, BundleWriter};

pub fn leaked(bytes: Vec<u8>) -> &'static [u8] {
    Box::leak(bytes.into_boxed_slice())
}

pub fn written() -> &'static [u8] {
    leaked(
        BundleWriter::new()
            .with_file("texts/en.json", "{\"a\": 1}")
            .with_file("names/empty.txt", "")
            .with_file("atlas.raw", vec![7u8; 4096])
            .finish(),
    )
}

/// Loads a named bundle fixture from `tests/fixtures`. `rust.bundle` is written by `BundleWriter`; `dotnet.bundle` is
/// written by C# `AssetBundle.BundleFile`. Both contain the files from `written`.
pub fn fixture(name: &str) -> &'static [u8] {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name);

    leaked(std::fs::read(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display())))
}

/// Returns `texts/en.json`, `names/empty.txt`, and `atlas.raw` from `bundle`, in that order.
pub fn contents(bundle: &Bundle) -> (Option<&str>, Option<&str>, Option<&[u8]>) {
    (
        bundle.text("texts/en.json"),
        bundle.text("names/empty.txt"),
        bundle.file("atlas.raw"),
    )
}
