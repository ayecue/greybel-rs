//! Rust-only tests for `memory` URIs. Like `file` URIs, they use absolute paths and an empty serialized authority.

use vscode_uri::Uri;

#[test]
fn should_build_a_memory_uri_like_a_file_uri() {
    let path = "/project/My File#1.src";
    let uri = Uri::memory(path);

    assert_eq!(uri.scheme(), "memory");
    assert_eq!(uri.authority(), "");
    assert_eq!(uri.path(), "/project/My File#1.src");
    assert_eq!(uri.to_string(), "memory:///project/My%20File%231.src");
}

#[test]
fn should_round_trip_a_memory_uri_through_its_string() {
    let uri = Uri::memory("/project/My File#1.src");
    let printed = uri.to_string();
    let parsed = Uri::parse(&printed).expect("parses");

    assert_eq!(parsed, uri);
}

#[test]
fn should_root_a_relative_memory_path() {
    let path = "a.src";
    let uri = Uri::memory(path);

    assert_eq!(uri.path(), "/a.src");
}

#[test]
fn should_root_a_relative_memory_uri_on_parse() {
    let source = "memory:a.src";
    let uri = Uri::parse(source).expect("parses");

    assert_eq!(uri.path(), "/a.src");
}

#[test]
fn should_print_an_empty_memory_path_as_the_root() {
    let path = "";
    let uri = Uri::memory(path);

    assert_eq!(uri.to_string(), "memory:///");
}

#[test]
fn should_take_the_dirname_of_a_memory_uri() {
    let main = Uri::memory("/project/main.src");
    let folder = main.dirname().expect("valid");

    assert_eq!(folder.to_string(), "memory:///project");
}

#[test]
fn should_join_paths_onto_a_memory_folder() {
    let folder = Uri::memory("/project/main.src").dirname().expect("valid");
    let joined = folder.join_path(&["../lib", "a.src"]).expect("valid");

    assert_eq!(joined, Uri::memory("/lib/a.src"));
}

#[test]
fn should_label_local_documents_as_paths() {
    let uris = [
        Uri::memory("/project/main.src"),
        Uri::file("/home/a b/x.src"),
        Uri::parse("vscode-vfs://github/owner/repo/main.src").expect("parses"),
        Uri::parse("file://server/share/x.src").expect("parses"),
    ];
    let [memory, file, remote, share] = uris.each_ref().map(Uri::label);

    assert_eq!(memory, "/project/main.src");
    if !cfg!(windows) {
        assert_eq!(file, "/home/a b/x.src");
    }

    assert_eq!(remote, "vscode-vfs://github/owner/repo/main.src");
    assert_eq!(share, "file://server/share/x.src");
}
