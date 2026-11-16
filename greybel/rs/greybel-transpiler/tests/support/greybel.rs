//! Greybel fixtures: `scripts/` and `special/` are transpiled; `dependencies/` contains imported, included, and
//! injected files.

use std::path::Path;

use greybel_documents_fs_provider::FsProvider;
use greybel_transpiler::{
    BeautifyOptions, BuildMode, TranspileError, TranspileOptions, Transpiler, UglifyOptions,
};

use crate::support::{BEAUTIFY_CUSTOM, Fixtures};

pub fn fixtures() -> Fixtures {
    Fixtures::new("greybel")
}

pub fn options(mode: BuildMode) -> TranspileOptions {
    fixtures().options(mode)
}

pub fn modes() -> [(&'static str, TranspileOptions); 4] {
    let custom = TranspileOptions {
        dev_mode: true,
        ..options(BuildMode::Beautify(BEAUTIFY_CUSTOM))
    };

    [
        ("default", options(BuildMode::Default)),
        (
            "uglify",
            options(BuildMode::Uglify(UglifyOptions::default())),
        ),
        (
            "beautify",
            options(BuildMode::Beautify(BeautifyOptions::default())),
        ),
        ("beautify, dev mode, keep parentheses, 10 spaces", custom),
    ]
}

pub fn project(path: &Path, options: &TranspileOptions) -> Result<String, TranspileError> {
    Transpiler::new(options)
        .transpile_project(&FsProvider::uri(path), &FsProvider::default())
        .map(|outputs| fixtures().outputs(&outputs))
}
