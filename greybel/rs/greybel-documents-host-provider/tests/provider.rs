//! Tests `HostProvider` reads and path resolution through an editor on the same thread.

use duplex_core::ErrorCode;
use greybel_core::{Dialect, ParserOptions};
use greybel_documents::{AsyncResourceProvider, DocumentOptions, ModuleGraph};

use crate::support::{
    FILES, connected, connected_to_a_failing_editor, connected_with_extensions, uri,
};

#[test]
fn should_read_a_file_by_uri() {
    let host = connected(FILES);
    let file = uri("file:///work/My%20Folder/a.src");

    let content = host.drive(host.provider.read(&file)).expect("reads");

    assert_eq!(file.path(), "/work/My Folder/a.src");
    assert_eq!(content.as_deref(), Some("a = 1"));
}

#[test]
fn should_read_nothing_for_a_missing_file_by_uri() {
    let host = connected(FILES);

    host.drive(host.provider.read(&uri("file:///work/My%20Folder/a.src")))
        .expect("reads");

    let missing = host
        .drive(host.provider.read(&uri("file:///work/missing.src")))
        .expect("reads");

    assert_eq!(missing, None);
    assert_eq!(
        host.files.reads(),
        ["file:///work/My%20Folder/a.src", "file:///work/missing.src"]
    );
}

#[test]
fn should_try_each_extension_for_an_import() {
    let host = connected(FILES);

    let lib = host
        .drive(
            host.provider
                .relative_to(&uri("file:///work/main.src"), "lib"),
        )
        .expect("resolves");

    assert_eq!(lib, uri("file:///work/lib.src"));
}

#[test]
fn should_read_the_found_import_once() {
    let host = connected(FILES);
    let lib = host
        .drive(
            host.provider
                .relative_to(&uri("file:///work/main.src"), "lib"),
        )
        .expect("resolves");

    let content = host.drive(host.provider.read(&lib)).expect("reads");

    assert_eq!(content.as_deref(), Some("lib = 1"));
    assert_eq!(
        host.files.reads(),
        ["file:///work/lib", "file:///work/lib.src"]
    );
}

#[test]
fn should_start_absolute_imports_at_the_workspace_folder() {
    let host = connected(FILES);

    let lib = host
        .drive(
            host.provider
                .relative_to(&uri("file:///work/a/b.src"), "/lib.src"),
        )
        .expect("resolves");

    assert_eq!(lib, uri("file:///work/lib.src"));
}

#[test]
fn should_take_entries_as_uris_or_paths() {
    let host = connected(FILES);
    let cases = [
        ("file:///work/main.src", "file:///work/main.src"),
        (
            "vscode-vfs://github/owner/main.src",
            "vscode-vfs://github/owner/main.src",
        ),
        ("C:\\work\\main.src", "file:///c%3A/work/main.src"),
        ("c:/work/main.src", "file:///c%3A/work/main.src"),
    ];

    let outcomes: Vec<_> = cases
        .iter()
        .map(|(text, _)| host.drive(host.provider.resolve(text)).expect("resolves"))
        .collect();

    for ((text, expected), entry) in cases.iter().zip(&outcomes) {
        assert_eq!(entry.to_string(), *expected, "{text}");
    }
}

#[test]
fn should_index_the_files_of_the_host() {
    let host = connected_with_extensions();

    let found = host
        .drive(host.provider.index("**/*.src", None))
        .expect("finds");

    assert_eq!(found, 3);
}

#[test]
fn should_resolve_an_indexed_import_without_reading() {
    let host = connected_with_extensions();

    host.drive(host.provider.index("**/*.src", None))
        .expect("finds");

    let main = uri("file:///work/main.src");

    let lib = host
        .drive(host.provider.relative_to(&main, "lib.src"))
        .expect("resolves");

    assert_eq!(lib, uri("file:///work/lib.src"));
    assert_eq!(host.files.reads(), Vec::<String>::new());
}

#[test]
fn should_read_a_deleted_indexed_file_from_the_host() {
    let host = connected_with_extensions();

    host.drive(host.provider.index("**/*.src", None))
        .expect("finds");

    let main = uri("file:///work/main.src");
    let lib = host
        .drive(host.provider.relative_to(&main, "lib.src"))
        .expect("resolves");

    host.provider.record_deletion(&lib);

    let lib = host
        .drive(host.provider.relative_to(&main, "lib.src"))
        .expect("resolves");

    assert_eq!(lib, uri("file:///work/lib.src"));
    assert_eq!(host.files.reads(), ["file:///work/lib.src"]);
}

#[test]
fn should_fail_a_read_the_host_fails() {
    let (provider, queue, _editor) = connected_to_a_failing_editor();

    let error = queue
        .drive(provider.read(&uri("file:///a.src")))
        .expect_err("fails");

    assert_eq!(error.to_string(), ErrorCode::DroppedMessage.message());
}

#[test]
fn should_fail_a_resolution_the_host_fails() {
    let (provider, queue, _editor) = connected_to_a_failing_editor();

    queue
        .drive(provider.read(&uri("file:///a.src")))
        .expect_err("fails");

    let error = queue
        .drive(provider.relative_to(&uri("file:///main.src"), "a"))
        .expect_err("fails");

    assert_eq!(error.to_string(), ErrorCode::DroppedMessage.message());
}

#[test]
fn should_load_a_program_through_the_host() {
    let host = connected(FILES);
    let options = DocumentOptions {
        parser: ParserOptions::new(Dialect::Greybel),
        ..DocumentOptions::default()
    };

    let graph = host
        .drive(ModuleGraph::load_async(
            &uri("file:///work/main.src"),
            &options,
            &host.provider,
        ))
        .expect("loads");

    let entry = graph.module(graph.entry().expect("has an entry"));

    assert_eq!(entry.uri, uri("file:///work/main.src"));

    let lib = graph.module(entry.dependencies()[0].module);

    assert_eq!(lib.uri, uri("file:///work/lib.src"));
}
