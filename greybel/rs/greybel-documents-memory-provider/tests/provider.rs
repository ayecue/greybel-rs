use greybel_documents::{PathResolution, ResourceProvider, Uri};
use greybel_documents_memory_provider::*;

use crate::support::provider::{files_in_lib, provider_with_a_replaced_file, relative};

#[test]
fn should_normalize_paths() {
    let cases = [
        ("main.src", "/main.src"),
        ("/lib/./a/../b.src", "/lib/b.src"),
        ("../../x", "/x"),
        ("/a//b/", "/a/b"),
    ];
    let outcomes = cases.map(|(path, _)| MemoryProvider::uri(path));

    for ((path, expected), uri) in cases.iter().zip(outcomes) {
        assert_eq!(uri, Uri::memory(expected), "{path}");
    }
}

#[test]
fn should_list_the_normalized_paths() {
    let provider = files_in_lib();
    let paths = provider.paths().collect::<Vec<_>>();

    assert_eq!(paths, ["/lib/a.src", "/lib/b"]);
}

#[test]
fn should_resolve_a_path_like_the_file_system() {
    let provider = files_in_lib();
    let resolved = ResourceProvider::resolve(&provider, "lib/../main").to_string();

    assert_eq!(resolved, "memory:///main");
}

#[test]
fn should_find_a_relative_file_like_the_file_system() {
    let provider = files_in_lib();
    let targets = ["a", "b", "../c"];
    let [extension_added, exact, missing] =
        targets.map(|target| relative(&provider, "/lib/main.src", target));

    assert_eq!(extension_added, "memory:///lib/a.src");
    assert_eq!(exact, "memory:///lib/b");
    assert_eq!(missing, "memory:///c.src");
}

#[test]
fn should_read_only_files_it_holds() {
    let provider = files_in_lib();
    let uris = [
        MemoryProvider::uri("/lib/a.src"),
        MemoryProvider::uri("/missing.src"),
        Uri::file("/lib/a.src"),
    ];
    let [held, missing, elsewhere] =
        uris.map(|uri| ResourceProvider::read(&provider, &uri).ok().flatten());

    assert_eq!(held.as_deref(), Some("a"));
    assert_eq!(missing, None);
    assert_eq!(elsewhere, None);
}

#[test]
fn should_append_the_configured_extensions() {
    let mut provider = MemoryProvider::with_resolution(PathResolution::new(["ms", "src"]));

    provider.insert("/lib.src", "a");

    let targets = ["lib", "missing"];
    let [found, missing] = targets.map(|target| relative(&provider, "/main.ms", target));

    assert_eq!(found, "memory:///lib.src");
    assert_eq!(missing, "memory:///missing.ms");
}

#[test]
fn should_have_no_previous_content_on_a_first_insert() {
    let mut provider = MemoryProvider::new();
    let previous = provider.insert("/a.src", "1");

    assert_eq!(previous, None);
}

#[test]
fn should_return_the_replaced_content_on_insert() {
    let mut provider = MemoryProvider::new();

    provider.insert("/a.src", "1");

    let previous = provider.insert("a.src", "2");

    assert_eq!(previous.as_deref(), Some("1"));
}

#[test]
fn should_hold_the_latest_inserted_content() {
    let provider = provider_with_a_replaced_file();
    let content = provider.content("/a.src");

    assert_eq!(content, Some("2"));
}

#[test]
fn should_return_the_removed_content() {
    let mut provider = provider_with_a_replaced_file();
    let _ = provider.content("/a.src");
    let removed = provider.remove("./a.src");

    assert_eq!(removed.as_deref(), Some("2"));
}

#[test]
fn should_not_contain_a_removed_file() {
    let mut provider = provider_with_a_replaced_file();

    provider.remove("./a.src");

    let contained = provider.contains("/a.src");

    assert!(!contained);
}
