mod descriptions;
mod signatures;
mod variations;

use std::sync::LazyLock;

use asset_bundle::Bundle;
use meta_utils::{Container, DEFAULT_LANGUAGE};

/// The `data/` directory embedded by build.rs.
static DATA: LazyLock<Bundle> = LazyLock::new(|| {
    Bundle::read(include_bytes!(concat!(env!("OUT_DIR"), "/data.bundle")))
        .expect("build.rs bundles data/")
});

/// Returns an unpacked `data/` file by relative path; the caller drops it after parsing.
pub(crate) fn text(path: &str) -> String {
    DATA.unpack_text(path)
        .unwrap_or_else(|| panic!("data/{path} is not bundled"))
}

pub(crate) fn load() -> Container {
    let mut container = Container::new();

    for group in variations::groups() {
        container.add_variations(group.name, group.variations);
    }

    for file in signatures::SIGNATURES {
        let path = format!("signatures/{file}.json");

        if let Err(error) = container.add_signature_json(&text(&path)) {
            panic!("data/{path}: {error}");
        }
    }

    for (file, type_name) in descriptions::DESCRIPTIONS_EN {
        let path = format!("descriptions/{DEFAULT_LANGUAGE}/{file}.json");

        if let Err(error) =
            container.add_descriptions_json(DEFAULT_LANGUAGE, type_name, &text(&path))
        {
            panic!("data/{path}: {error}");
        }
    }

    container
}
