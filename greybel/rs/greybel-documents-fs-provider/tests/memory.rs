//! Tests in-memory file overrides taking precedence over filesystem reads and glob matches.

use std::collections::BTreeMap;
use std::sync::Arc;

use greybel_documents::{PathResolution, ResourceProvider};
use greybel_documents_fs_provider::FsProvider;

#[test]
fn should_read_and_find_files_in_memory() {
    let folder = std::env::temp_dir().join("greybel-fs-provider-memory");
    let entry = FsProvider::uri(&folder.join("main.src"));
    let library = FsProvider::uri(&folder.join("lib.src"));
    let provider = FsProvider::new(PathResolution::new(["src".to_owned()])).with_memory(Arc::new(
        BTreeMap::from([
            (entry.clone(), "import_code(\"lib\")".to_owned()),
            (library.clone(), "x = 1".to_owned()),
        ]),
    ));

    assert_eq!(
        provider.read(&entry).expect("reads"),
        Some("import_code(\"lib\")".to_owned())
    );
    assert_eq!(provider.relative_to(&entry, "lib"), library);
}
