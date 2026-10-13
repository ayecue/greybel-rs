use std::sync::LazyLock;

use asset_bundle::Bundle;
use text_mesh_setup::Setup;

/// Embedded terminal setup JSON, raw atlas and sprite pixels, and source fonts.
pub(crate) static BUNDLE: LazyLock<Bundle> = LazyLock::new(|| {
    Bundle::read(include_bytes!("../assets.bundle")).expect("assets.bundle is a bundle")
});

pub(crate) static SETUP: LazyLock<Setup> = LazyLock::new(|| {
    Setup::read(text("text-mesh.json")).expect("text-mesh.json is the game's setup")
});

/// Returns embedded file `name`.
///
/// # Panics
///
/// Panics when `name` is absent from the bundle.
pub(crate) fn file(name: &str) -> &'static [u8] {
    BUNDLE
        .file(name)
        .unwrap_or_else(|| panic!("assets.bundle has no {name}"))
}

pub(crate) fn text(name: &str) -> &'static str {
    BUNDLE
        .text(name)
        .unwrap_or_else(|| panic!("assets.bundle has no {name}"))
}

/// Returns raw pixels for setup image `<name>.png` from `<folder>/<stem>.<raw>`.
pub(crate) fn pixels(folder: &str, image: &str, raw: &str) -> &'static [u8] {
    let stem = image.strip_suffix(".png").unwrap_or(image);

    file(&format!("{folder}/{stem}.{raw}"))
}
