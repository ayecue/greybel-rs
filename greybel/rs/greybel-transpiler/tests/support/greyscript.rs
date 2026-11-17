//! GreyScript fixtures: `scripts/` and `special/` are transpiled; `dependencies/` contains imported, included,
//! injected, and loaded files.

use greybel_core::Dialect;
use greybel_documents::Uri;
use greybel_transpiler::{
    BeautifyOptions, BuildMode, PathMapping, TranspileOptions, UglifyOptions,
};

use crate::support::{BEAUTIFY_CUSTOM, Fixtures};

pub fn fixtures() -> Fixtures {
    Fixtures::new("greyscript")
}

/// Returns reference-matching GreyScript options: intrinsics are never renamed, and `import_code` paths map under
/// `tests/fixtures/greyscript/` with a leading slash.
pub fn options(mode: BuildMode) -> TranspileOptions {
    TranspileOptions {
        dialect: Dialect::GreyScript,
        path_mapping: Some(PathMapping {
            local_root: Uri::file(&fixtures().path().display().to_string()),
            ingame_directory: String::new(),
        }),
        ..fixtures().options(mode)
    }
}

pub fn dev_mode(options: TranspileOptions) -> TranspileOptions {
    TranspileOptions {
        dev_mode: true,
        ..options
    }
}

pub fn modes() -> [(&'static str, TranspileOptions); 5] {
    let beautify = || options(BuildMode::Beautify(BeautifyOptions::default()));

    [
        ("default", options(BuildMode::Default)),
        (
            "uglify",
            options(BuildMode::Uglify(UglifyOptions::default())),
        ),
        ("beautify", beautify()),
        ("beautify, dev mode", dev_mode(beautify())),
        (
            "beautify, dev mode, keep parentheses, 10 spaces",
            dev_mode(options(BuildMode::Beautify(BEAUTIFY_CUSTOM))),
        ),
    ]
}
