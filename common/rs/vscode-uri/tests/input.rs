//! Rust-only tests for parsing document input as a URI or path; one-letter schemes are treated as Windows drives.

use vscode_uri::Uri;

#[test]
fn should_require_at_least_two_characters_for_a_scheme() {
    let texts = [
        "file:///project/main.src",
        "memory:///main.src",
        "vscode-vfs://github/owner/repo",
        "git+ssh://host/repo",
        "C:\\project\\main.src",
        "c:/project/main.src",
        "/project/main.src",
        "main.src",
        "./a:b.src",
        "1x:/a",
    ];
    let [
        file,
        memory,
        vfs,
        git_ssh,
        backslash_drive,
        slash_drive,
        absolute,
        relative,
        dotted,
        digit,
    ] = texts.map(Uri::has_scheme);

    assert!(file);
    assert!(memory);
    assert!(vfs);
    assert!(git_ssh);
    assert!(!backslash_drive);
    assert!(!slash_drive);
    assert!(!absolute);
    assert!(!relative);
    assert!(!dotted);
    assert!(!digit);
}

#[test]
fn should_recognise_drive_paths() {
    let texts = [
        "C:\\project\\main.src",
        "d:/project/main.src",
        "C:",
        "C:main.src",
        "file:///c:/project",
        "\\\\server\\share",
    ];
    let [backslashes, slashes, bare, drive_relative, uri, unc] = texts.map(Uri::is_drive_path);

    assert!(backslashes);
    assert!(slashes);
    assert!(bare);
    assert!(!drive_relative);
    assert!(!uri);
    assert!(!unc);
}

#[test]
fn should_turn_drive_paths_into_file_uris_on_every_platform() {
    let expected = "file:///c%3A/project/main.src";
    let texts = [
        "C:\\project\\main.src",
        "C:/project/main.src",
        "file:///c%3A/project/main.src",
        "file:///C:/project/main.src",
    ];
    let outcomes = texts.map(|text| Uri::from_path_or_uri(text).expect("valid"));

    for (text, uri) in texts.iter().zip(&outcomes) {
        assert_eq!(uri.to_string(), expected, "{text}");
    }
}

#[test]
fn should_keep_the_scheme_of_a_uri() {
    let text = "vscode-vfs://github/owner/repo/main.src";
    let remote = Uri::from_path_or_uri(text).expect("valid");

    assert_eq!(remote.scheme(), "vscode-vfs");
    assert_eq!(remote.path(), "/owner/repo/main.src");
}

#[test]
fn should_turn_an_absolute_path_into_a_file_uri() {
    if cfg!(windows) {
        return;
    }

    let text = "/home/a b/main.src";
    let expected = Uri::file("/home/a b/main.src");
    let uri = Uri::from_path_or_uri(text).expect("valid");

    assert_eq!(uri, expected);
}

#[test]
fn should_validate_text_with_a_scheme_as_a_uri() {
    let texts = ["http://host", "foo:////double"];
    let [http, double_slash] = texts.map(Uri::from_path_or_uri);

    assert!(http.is_ok());
    assert!(double_slash.is_err());
}
