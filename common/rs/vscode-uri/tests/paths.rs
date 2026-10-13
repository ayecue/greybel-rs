//! Port of vscode-uri 3.2.0's `paths.test.ts`. `Utils` operations are `Uri` methods; `PosixPath` covers cases that
//! upstream delegates to Node.

use support::parse;
use vscode_uri::{PosixPath, Uri, UriComponentContainer};

use crate::support;

#[test]
fn should_join_paths_like_node() {
    let cases: [(&str, &[&str], &str, bool); 9] = [
        ("foo://a/foo/bar", &["x"], "foo://a/foo/bar/x", true),
        ("foo://a/foo/bar/", &["x"], "foo://a/foo/bar/x", true),
        ("foo://a/foo/bar/", &["/x"], "foo://a/foo/bar/x", true),
        ("foo://a/foo/bar/", &["x/"], "foo://a/foo/bar/x/", true),
        ("foo://a/foo/bar/", &["x", "y"], "foo://a/foo/bar/x/y", true),
        (
            "foo://a/foo/bar/",
            &["x/", "/y"],
            "foo://a/foo/bar/x/y",
            true,
        ),
        ("foo://a/foo/bar/", &[".", "/y"], "foo://a/foo/bar/y", true),
        (
            "foo://a/foo/bar/",
            &["x/y/z", ".."],
            "foo://a/foo/bar/x/y",
            true,
        ),
        (
            "untitled:untitled-1",
            &["..", "untitled-2"],
            "untitled:untitled-2",
            true,
        ),
    ];
    let outcomes: Vec<_> = cases
        .iter()
        .map(|(uri, paths, _, against_node)| {
            let base = parse(uri);
            let joined = base.join_path(paths).expect("valid").to_string();
            let posix = against_node.then(|| {
                let root = if base.path().is_empty() {
                    "/"
                } else {
                    base.path()
                };
                let mut all = vec![root];

                all.extend_from_slice(paths);

                PosixPath::join(&all)
            });

            (joined, posix)
        })
        .collect();

    for ((uri, _, expected, _), (joined, posix)) in cases.iter().zip(&outcomes) {
        assert_eq!(joined, expected, "{uri}");
        if let Some(posix) = posix {
            assert_eq!(posix, parse(expected).path(), "{uri} (posix)");
        }
    }
}

#[test]
fn should_resolve_paths_like_node() {
    let cases = [
        ("foo://a/foo/bar", "x", "foo://a/foo/bar/x", true),
        ("foo://a/foo/bar/", "x", "foo://a/foo/bar/x", true),
        ("foo://a/foo/bar/", "/x", "foo://a/x", true),
        ("foo://a/foo/bar/", "x/", "foo://a/foo/bar/x", true),
        ("foo://a", "x/", "foo://a/x", true),
        ("foo://a", "/x/", "foo://a/x", true),
        ("foo://a/b", "/x/..//y/.", "foo://a/y", true),
        ("foo://a/b", "x/..//y/.", "foo://a/b/y", true),
        ("untitled:untitled-1", "../foo", "untitled:foo", false),
        ("untitled:", "foo", "untitled:foo", false),
        ("untitled:", "..", "untitled:", false),
        ("untitled:", "/foo", "untitled:foo", false),
        ("untitled:/", "/foo", "untitled:/foo", false),
    ];
    let outcomes: Vec<_> = cases
        .iter()
        .map(|(uri, path, _, against_node)| {
            let base = parse(uri);
            let resolved = base.resolve_path(&[path]).expect("valid").to_string();
            let posix = against_node.then(|| {
                let root = if base.path().is_empty() {
                    "/"
                } else {
                    base.path()
                };

                PosixPath::resolve(&[root, path])
            });

            (resolved, posix)
        })
        .collect();

    for ((uri, _, expected, _), (resolved, posix)) in cases.iter().zip(&outcomes) {
        assert_eq!(resolved, expected, "{uri}");
        if let Some(posix) = posix {
            assert_eq!(posix, parse(expected).path(), "{uri} (posix)");
        }
    }
}

#[test]
fn should_normalize_paths_like_node() {
    let cases = [
        ("a", "a"),
        ("/a", "/a"),
        ("a/", "a/"),
        ("a/b", "a/b"),
        ("/a/foo/bar/x", "/a/foo/bar/x"),
        ("/a/foo/bar//x", "/a/foo/bar/x"),
        ("/a/foo/bar///x", "/a/foo/bar/x"),
        ("/a/foo/bar/x/", "/a/foo/bar/x/"),
        ("a/foo/bar/x/", "a/foo/bar/x/"),
        ("a/foo/bar/x//", "a/foo/bar/x/"),
        ("//a/foo/bar/x//", "/a/foo/bar/x/"),
        ("a/.", "a"),
        ("a/..", "."),
        ("a/./b", "a/b"),
        ("a/././b", "a/b"),
        ("a/n/../b", "a/b"),
        ("a/n/../", "a/"),
        ("/a/n/../..", "/"),
        ("/a/n/../../..", "/"),
        ("..", ".."),
        ("/..", "/"),
        ("untitled-1/foo/bar/.", "untitled-1/foo/bar"),
    ];
    let outcomes: Vec<_> = cases
        .iter()
        .map(|(path, _)| {
            let uri = Uri::from_components(UriComponentContainer {
                scheme: "foo".to_owned(),
                authority: if path.starts_with('/') { "bar" } else { "" }.to_owned(),
                path: (*path).to_owned(),
                ..UriComponentContainer::default()
            })
            .expect("valid");
            let joined = uri.join_path(&[]).expect("valid").path().to_owned();

            (joined, PosixPath::normalize(path))
        })
        .collect();

    for ((path, expected), (joined, posix)) in cases.iter().zip(&outcomes) {
        assert_eq!(joined, expected, "{path}");
        assert_eq!(posix, expected, "{path} (posix)");
    }
}

#[test]
fn should_take_the_extname_like_node() {
    let cases = [
        ("foo://a/foo/bar", ""),
        ("foo://a/foo/bar.foo", ".foo"),
        ("foo://a/foo/.foo", ""),
        ("foo://a/foo/a.foo/", ".foo"),
        ("foo://a/foo/a.foo//", ".foo"),
        ("untitled:untitled-1", ""),
    ];
    let outcomes: Vec<_> = cases
        .iter()
        .map(|(input, _)| {
            (
                parse(input).extname().to_owned(),
                PosixPath::extname(input).to_owned(),
            )
        })
        .collect();

    for ((input, expected), (extname, posix)) in cases.iter().zip(&outcomes) {
        assert_eq!(extname, expected, "{input}");
        assert_eq!(posix, expected, "{input} (posix)");
    }
}

#[test]
fn should_take_the_basename_like_node() {
    let cases = [
        ("foo://a/some/file/test.txt", "test.txt"),
        ("foo://a/some/file/", "file"),
        ("foo://a/some/file///", "file"),
        ("foo://a/some/file", "file"),
        ("foo://a/some", "some"),
        ("foo://a/", ""),
        ("foo://a", ""),
        ("untitled:untitled-1", "untitled-1"),
    ];
    let outcomes: Vec<_> = cases
        .iter()
        .map(|(input, _)| {
            let uri = parse(input);

            (
                uri.basename().to_owned(),
                PosixPath::basename(uri.path()).to_owned(),
            )
        })
        .collect();

    for ((input, expected), (basename, posix)) in cases.iter().zip(&outcomes) {
        assert_eq!(basename, expected, "{input}");
        assert_eq!(posix, expected, "{input} (posix)");
    }
}

#[test]
fn should_take_the_dirname_like_node() {
    let cases = [
        ("foo://a/some/file/test.txt", "foo://a/some/file", true),
        ("foo://a/some/file/", "foo://a/some", true),
        ("foo://a/some/file///", "foo://a/some", true),
        ("foo://a/some/file", "foo://a/some", true),
        ("foo://a/some", "foo://a/", true),
        ("foo://a/", "foo://a/", true),
        ("foo://a", "foo://a", false),
        ("foo://", "foo:", false),
        ("untitled:untitled-1", "untitled:", false),
    ];
    let outcomes: Vec<_> = cases
        .iter()
        .map(|(input, _, against_node)| {
            let uri = parse(input);
            let dirname = uri.dirname().expect("valid").to_string();
            let posix = against_node.then(|| PosixPath::dirname(uri.path()).to_owned());

            (dirname, posix)
        })
        .collect();

    for ((input, expected, _), (dirname, posix)) in cases.iter().zip(&outcomes) {
        assert_eq!(dirname, expected, "{input}");
        if let Some(posix) = posix {
            assert_eq!(posix, parse(expected).path(), "{input} (posix)");
        }
    }
}
