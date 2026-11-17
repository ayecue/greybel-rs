//! Tests `custom/imported-files`: the files a document imports, read through the client.

use greybel_lsp::ImportedFiles;
use serde_json::json;

use crate::support::{Client, uri};

#[test]
fn should_list_the_files_a_document_imports_directly_and_through_others() {
    let mut client = Client::started(&[
        (
            "main.src",
            "import_code(\"lib/a.src\")\n#include \"missing.src\"\n#import util from \"lib/util.src\"",
        ),
        (
            "lib/a.src",
            "#include \"b.src\"\nimport_code(\"/main.src\")",
        ),
        ("lib/b.src", "x = 1"),
        ("lib/util.src", "module.exports = {}"),
        ("other.src", "print 1"),
    ]);

    let files = client.request::<ImportedFiles>(json!({ "uri": uri("main.src") }));

    assert_eq!(
        files,
        [uri("lib/a.src"), uri("lib/util.src"), uri("lib/b.src")]
    );
}

#[test]
fn should_list_nothing_for_a_document_without_imports() {
    let mut client = Client::started(&[("main.src", "print 1")]);

    let files = client.request::<ImportedFiles>(json!({ "uri": uri("main.src") }));

    assert!(files.is_empty(), "{files:?}");
}
