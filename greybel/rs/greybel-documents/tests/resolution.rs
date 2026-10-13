use std::convert::Infallible;
use std::future::ready;

use greybel_documents::PathResolution;
use greybel_test_support::future::ready_output;
use vscode_uri::Uri;

use crate::support::uri;

#[test]
fn should_resolve_asynchronously_to_the_same_uris() {
    let resolution = PathResolution::new(["src", "ms"]);
    let files = ["/a", "/b.ms", "/c.src", "/c.ms", "/x.src.src"].map(uri);
    let targets = ["/a", "/b", "/c", "/gone", "/gone.src", "/gone.ms", "/x.src"].map(uri);
    let expected: Vec<_> = targets
        .iter()
        .map(|target| resolution.resolve(target, |uri| files.contains(uri)))
        .collect();
    let outcomes: Vec<_> = targets
        .iter()
        .map(|target| {
            ready_output(resolution.resolve_async(target, |uri| {
                ready(Ok::<_, Infallible>(files.contains(&uri)))
            }))
        })
        .collect();

    for ((target, resolved), expected) in targets.iter().zip(outcomes).zip(expected) {
        assert_eq!(resolved, Ok(expected), "{target}");
    }
}

#[test]
fn should_try_the_uri_then_each_extension() {
    let resolution = PathResolution::new(["src", "ms"]);
    let files = ["/a", "/b.ms", "/c.src", "/c.ms"].map(uri);
    let paths = ["/a", "/b", "/c", "/gone", "/gone.src", "/gone.ms"];
    let [plain, ms, src, gone, gone_src, gone_ms] = paths.map(|path| {
        resolution
            .resolve(&uri(path), |uri| files.contains(uri))
            .path()
            .to_owned()
    });

    assert_eq!(plain, "/a");
    assert_eq!(ms, "/b.ms");
    assert_eq!(src, "/c.src");
    assert_eq!(gone, "/gone.src");
    assert_eq!(gone_src, "/gone.src");
    assert_eq!(gone_ms, "/gone.ms");
}

#[test]
fn should_take_a_doubled_extension_when_only_it_exists() {
    let resolution = PathResolution::default();
    let files = [uri("/x.src.src")];
    let resolved = resolution.resolve(&uri("/x.src"), |uri| files.contains(uri));

    assert_eq!(resolved, uri("/x.src.src"));
}

#[test]
fn should_keep_a_written_extension_of_a_missing_file() {
    let resolution = PathResolution::default();
    let resolved = resolution.resolve(&uri("/later.src"), |_| false);

    assert_eq!(resolved, uri("/later.src"));
}

#[test]
fn should_join_imports_relative_to_the_source_or_its_folder() {
    let resolution = PathResolution::default().with_folders([uri("/work"), uri("/work/nested")]);
    let cases = [
        ("/work/a/main.src", "lib.src"),
        ("/work/a/main.src", "./x/../lib.src"),
        ("/work/a/main.src", "../../../lib.src"),
        ("/work/a/main.src", "/lib.src"),
        ("/work/nested/main.src", "/lib.src"),
        ("/elsewhere/main.src", "/lib.src"),
        ("/workshop/main.src", "/lib.src"),
    ];
    let [
        sibling,
        dotted,
        above_root,
        folder,
        nested_folder,
        outside,
        prefix_only,
    ] = cases.map(|(source, target)| resolution.join(&uri(source), target).path().to_owned());

    assert_eq!(sibling, "/work/a/lib.src");
    assert_eq!(dotted, "/work/a/lib.src");
    assert_eq!(above_root, "/lib.src");
    assert_eq!(folder, "/work/lib.src");
    assert_eq!(nested_folder, "/work/nested/lib.src");
    assert_eq!(outside, "/lib.src");
    assert_eq!(prefix_only, "/lib.src");
}

#[test]
fn should_keep_the_scheme_and_authority_of_the_source() {
    let resolution = PathResolution::default()
        .with_folders([Uri::parse("vscode-vfs://github/owner/repo").expect("parses")]);
    let source = Uri::parse("vscode-vfs://github/owner/repo/src/main.src").expect("parses");
    let targets = ["../lib/a.src", "/x.src"];
    let [relative, rooted] = targets.map(|target| resolution.join(&source, target).to_string());

    assert_eq!(relative, "vscode-vfs://github/owner/repo/lib/a.src");
    assert_eq!(rooted, "vscode-vfs://github/owner/repo/x.src");
}

#[test]
fn should_find_no_folder_for_a_uri_of_another_scheme() {
    let resolution = PathResolution::default()
        .with_folders([Uri::parse("vscode-vfs://github/owner/repo").expect("parses")]);
    let folder = resolution.folder_of(&uri("/owner/repo/main.src"));

    assert!(folder.is_none());
}

#[test]
fn should_give_the_path_of_a_uri_in_a_folder() {
    let folder = uri("/work");
    let uris = ["/work/lib/a.src", "/work", "/workspace/a.src", "/a.src"].map(uri);
    let [nested, itself, prefix_only, outside] = uris
        .each_ref()
        .map(|uri| PathResolution::path_in(&folder, uri));

    assert_eq!(nested, Some("lib/a.src"));
    assert_eq!(itself, Some(""));
    assert_eq!(prefix_only, None);
    assert_eq!(outside, None);
}

#[test]
fn should_give_no_path_of_a_uri_of_another_scheme() {
    let folder = Uri::parse("vscode-vfs://github/owner/repo").expect("parses");
    let file = uri("/owner/repo/main.src");

    assert_eq!(PathResolution::path_in(&folder, &file), None);
}
