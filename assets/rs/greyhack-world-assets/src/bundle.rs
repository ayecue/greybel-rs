use std::sync::LazyLock;

use asset_bundle::Bundle;

/// Game files embedded by `greyhack-util` from `assets.bundle`, with content hashes in `fingerprint.json`.
pub(crate) static BUNDLE: LazyLock<Bundle> = LazyLock::new(|| {
    Bundle::read(include_bytes!("../assets.bundle")).expect("assets.bundle is a bundle")
});

/// Returns bundled text file `name`.
///
/// # Panics
///
/// Panics when `name` is absent from the bundle.
pub(crate) fn text(name: &str) -> &'static str {
    BUNDLE
        .text(name)
        .unwrap_or_else(|| panic!("assets.bundle has no {name}"))
}
