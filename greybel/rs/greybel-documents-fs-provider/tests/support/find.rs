//! Fixture project for glob-search tests.

use greybel_documents::{PathResolution, Uri};
use greybel_documents_fs_provider::FsProvider;
use greybel_test_support::TempFolder;

/// Creates a project with distinct paths for extension, `node_modules`, and hidden-file glob cases.
pub fn project() -> TempFolder {
    TempFolder::new(
        "greybel-documents-fs-provider",
        [
            ("main.src", ""),
            ("lib/util.gs", ""),
            ("notes.txt", ""),
            ("node_modules/dep/vendored.src", ""),
            (".config/hidden.src", ""),
        ],
    )
}

/// Creates a provider rooted at `folder` with `src` as its source extension.
pub fn provider(folder: &TempFolder) -> FsProvider {
    FsProvider::new(PathResolution::new(["src"]).with_folders([FsProvider::uri(folder.path())]))
}

/// Maps relative `paths` to file URIs under `folder`.
pub fn uris(folder: &TempFolder, paths: &[&str]) -> Vec<Uri> {
    paths
        .iter()
        .map(|path| FsProvider::uri(&folder.path().join(path)))
        .collect()
}
