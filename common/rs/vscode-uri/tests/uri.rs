//! Port of vscode-uri 3.2.0's `uri.test.ts`. Unlike VS Code's IPC form, `Uri` serializes to its URI string here.

use support::{assert_components, change, from, parse};
use vscode_uri::{Uri, UriChange};

use crate::support;

#[test]
fn should_print_a_file_uri_with_an_encoded_drive_letter() {
    let paths = ["c:/win/path", "C:/win/path", "c:/win/path/", "/c:/win/path"];
    let [lower, upper, trailing, slashed] = paths.map(|path| Uri::file(path).to_string());

    assert_eq!(lower, "file:///c%3A/win/path");
    assert_eq!(upper, "file:///c%3A/win/path");
    assert_eq!(trailing, "file:///c%3A/win/path/");
    assert_eq!(slashed, "file:///c%3A/win/path");
}

#[test]
fn should_treat_backslashes_as_separators_only_on_windows() {
    let paths = ["c:\\win\\path", "c:\\win/path"];
    let [backslashes, mixed] = paths.map(|path| Uri::file(path).to_string());

    if cfg!(windows) {
        assert_eq!(backslashes, "file:///c%3A/win/path");
        assert_eq!(mixed, "file:///c%3A/win/path");
    } else {
        assert_eq!(backslashes, "file:///c%3A%5Cwin%5Cpath");
        assert_eq!(mixed, "file:///c%3A%5Cwin/path");
    }
}

#[test]
fn should_give_the_fs_path_of_a_drive_letter_file() {
    let paths = [
        "c:\\win\\path",
        "c:\\win/path",
        "c:/win/path",
        "c:/win/path/",
        "C:/win/path",
        "/c:/win/path",
        "./c/win/path",
    ];
    let [
        backslashes,
        mixed,
        slashes,
        trailing,
        upper,
        slashed,
        relative,
    ] = paths.map(|path| Uri::file(path).fs_path());

    if cfg!(windows) {
        assert_eq!(backslashes, "c:\\win\\path");
        assert_eq!(mixed, "c:\\win\\path");
        assert_eq!(slashes, "c:\\win\\path");
        assert_eq!(trailing, "c:\\win\\path\\");
        assert_eq!(upper, "c:\\win\\path");
        assert_eq!(slashed, "c:\\win\\path");
        assert_eq!(relative, "\\.\\c\\win\\path");
    } else {
        assert_eq!(slashes, "c:/win/path");
        assert_eq!(trailing, "c:/win/path/");
        assert_eq!(upper, "c:/win/path");
        assert_eq!(slashed, "c:/win/path");
        assert_eq!(relative, "/./c/win/path");
    }
}

#[test]
fn should_give_a_root_fs_path_when_the_path_is_in_the_authority() {
    let source = "file://%2Fhome%2Fticino%2Fdesktop%2Fcpluscplus%2Ftest.cpp";
    let value = parse(source);

    assert_eq!(
        value.authority(),
        "/home/ticino/desktop/cpluscplus/test.cpp"
    );
    assert_eq!(value.path(), "/");
    assert_eq!(value.fs_path(), if cfg!(windows) { "\\" } else { "/" });
}

#[test]
fn should_print_http_uris() {
    let uris = [
        from("http", "www.msft.com", "/my/path", "", ""),
        from("http", "www.MSFT.com", "/my/path", "", ""),
        from("http", "", "my/path", "", ""),
        from("http", "", "/my/path", "", ""),
        from("http", "a-test-site.com", "/", "test=true", ""),
        from("http", "a-test-site.com", "/", "", "test=true"),
    ];
    let [lower, upper, relative, absolute, query, fragment] = uris.map(|uri| uri.to_string());

    assert_eq!(lower, "http://www.msft.com/my/path");
    assert_eq!(upper, "http://www.msft.com/my/path");
    assert_eq!(relative, "http:/my/path");
    assert_eq!(absolute, "http:/my/path");
    assert_eq!(query, "http://a-test-site.com/?test%3Dtrue");
    assert_eq!(fragment, "http://a-test-site.com/#test%3Dtrue");
}

#[test]
fn should_print_http_uris_unencoded() {
    let uris = [
        from("http", "a-test-site.com", "/", "test=true", ""),
        from("http", "a-test-site.com", "/", "", "test=true"),
        from("http", "", "/api/files/test.me", "t=1234", ""),
    ];
    let [query, fragment, no_authority] = uris.map(|uri| uri.to_unencoded_string());

    assert_eq!(query, "http://a-test-site.com/?test=true");
    assert_eq!(fragment, "http://a-test-site.com/#test=true");
    assert_eq!(no_authority, "http:/api/files/test.me?t=1234");
}

#[test]
fn should_decode_a_file_uri_with_an_umlaut_and_a_hash() {
    let source = "file://shares/pröjects/c%23/#l12";
    let value = parse(source);

    assert_eq!(value.authority(), "shares");
    assert_eq!(value.path(), "/pröjects/c#/");
    assert_eq!(value.fragment(), "l12");
    assert_eq!(value.to_string(), "file://shares/pr%C3%B6jects/c%23/#l12");
    assert_eq!(
        value.to_unencoded_string(),
        "file://shares/pröjects/c%23/#l12"
    );
}

#[test]
fn should_parse_the_encoded_and_unencoded_form_alike() {
    let value = parse("file://shares/pröjects/c%23/#l12");
    let forms = [value.to_unencoded_string(), value.to_string()];
    let [unencoded, encoded] = forms.map(|form| parse(&form));

    assert_eq!(unencoded, encoded);
}

#[test]
fn should_keep_a_uri_unchanged_without_changes() {
    let uri = parse("foo:bar/path");
    let changes = [
        UriChange::default(),
        change(Some("foo"), None, Some("bar/path")),
    ];
    let [empty, same] = changes.map(|change| uri.with(change).expect("valid"));

    assert_eq!(empty, uri);
    assert_eq!(same, uri);
}

#[test]
fn should_change_the_scheme_of_a_uri() {
    let uri = parse("before:some/file/path");
    let changed = uri
        .with(change(Some("after"), None, None))
        .expect("valid")
        .to_string();

    assert_eq!(changed, "after:some/file/path");
}

#[test]
fn should_print_a_changed_uri_for_every_scheme() {
    let cases = [
        ("http", "http:/api/files/test.me?t%3D1234"),
        ("https", "https:/api/files/test.me?t%3D1234"),
        ("HTTP", "HTTP:/api/files/test.me?t%3D1234"),
        ("HTTPS", "HTTPS:/api/files/test.me?t%3D1234"),
        ("boo", "boo:/api/files/test.me?t%3D1234"),
    ];
    let outcomes: Vec<_> = cases
        .iter()
        .map(|(scheme, _)| {
            from("s", "", "", "", "")
                .with(UriChange {
                    scheme: Some((*scheme).to_owned()),
                    authority: Some(String::new()),
                    path: Some("/api/files/test.me".to_owned()),
                    query: Some("t=1234".to_owned()),
                    fragment: Some(String::new()),
                })
                .expect("valid")
        })
        .collect();

    for ((_, expected), changed) in cases.iter().zip(&outcomes) {
        assert_eq!(changed.to_string(), *expected);
    }
}

#[test]
fn should_remove_components_set_to_empty() {
    let added = parse("scheme:/path")
        .with(change(None, Some("authority"), None))
        .expect("valid");
    let cases = [
        (parse("scheme://authority/path"), Some(""), None),
        (added.clone(), Some(""), None),
        (added, None, Some("")),
        (parse("scheme:/path"), Some(""), None),
    ];
    let [
        removed_authority,
        removed_added_authority,
        removed_path,
        nothing_to_remove,
    ] = cases.map(|(uri, authority, path)| {
        uri.with(change(None, authority, path))
            .expect("valid")
            .to_string()
    });

    assert_eq!(removed_authority, "scheme:/path");
    assert_eq!(removed_added_authority, "scheme:/path");
    assert_eq!(removed_path, "scheme://authority");
    assert_eq!(nothing_to_remove, "scheme:/path");
}

#[test]
fn should_reject_invalid_changes() {
    let uri = parse("foo:bar/path");
    let changes = [
        change(Some("fai:l"), None, None),
        change(Some("fäil"), None, None),
        change(None, Some("fail"), None),
        change(None, None, Some("//fail")),
    ];
    let [colon, umlaut, relative_path, double_slash] = changes.map(|change| uri.with(change));

    assert!(colon.is_err());
    assert!(umlaut.is_err());
    assert!(relative_path.is_err());
    assert!(double_slash.is_err());
}

#[test]
fn should_parse_the_components_of_a_uri() {
    let sources = [
        "http:/api/files/test.me?t=1234",
        "http://api/files/test.me?t=1234",
        "file:///c:/test/me",
        "file://shares/files/c%23/p.cs",
        "file:///c:/Source/Z%C3%BCrich%20or%20Zurich%20(%CB%88zj%CA%8A%C9%99r%C9%AAk,/Code/resources/app/plugins/c%23/plugin.json",
        "file:///c:/test %25/path",
        "inmemory:",
        "foo:api/files/test",
        "file:?q",
        "file:#d",
        "f3ile:#d",
        "foo+bar:path",
        "foo-bar:path",
        "foo.bar:path",
    ];
    let [
        no_authority,
        authority,
        drive,
        share,
        unicode,
        percent,
        scheme_only,
        relative,
        file_query,
        file_fragment,
        other_fragment,
        plus,
        dash,
        dot,
    ] = sources.map(parse);

    assert_components(
        &no_authority,
        ["http", "", "/api/files/test.me", "t=1234", ""],
    );
    assert_components(&authority, ["http", "api", "/files/test.me", "t=1234", ""]);
    assert_components(&drive, ["file", "", "/c:/test/me", "", ""]);
    assert_eq!(
        drive.fs_path(),
        if cfg!(windows) {
            "c:\\test\\me"
        } else {
            "c:/test/me"
        }
    );
    assert_components(&share, ["file", "shares", "/files/c#/p.cs", "", ""]);
    assert_eq!(
        share.fs_path(),
        if cfg!(windows) {
            "\\\\shares\\files\\c#\\p.cs"
        } else {
            "//shares/files/c#/p.cs"
        }
    );
    assert_components(
        &unicode,
        [
            "file",
            "",
            "/c:/Source/Zürich or Zurich (ˈzjʊərɪk,/Code/resources/app/plugins/c#/plugin.json",
            "",
            "",
        ],
    );
    assert_components(&percent, ["file", "", "/c:/test %/path", "", ""]);
    assert_components(&scheme_only, ["inmemory", "", "", "", ""]);
    assert_components(&relative, ["foo", "", "api/files/test", "", ""]);
    assert_components(&file_query, ["file", "", "/", "q", ""]);
    assert_components(&file_fragment, ["file", "", "/", "", "d"]);
    assert_components(&other_fragment, ["f3ile", "", "", "", "d"]);
    assert_components(&plus, ["foo+bar", "", "path", "", ""]);
    assert_components(&dash, ["foo-bar", "", "path", "", ""]);
    assert_components(&dot, ["foo.bar", "", "path", "", ""]);
}

#[test]
fn should_reject_a_double_slash_path_without_authority() {
    let source = "file:////shares/files/p.cs";
    let result = Uri::parse(source);

    assert!(result.is_err());
}

#[test]
fn should_turn_windows_and_unc_paths_into_file_uris() {
    if !cfg!(windows) {
        return;
    }

    let paths = [
        "c:\\test\\drive",
        "\\\\shäres\\path\\c#\\plugin.json",
        "\\\\localhost\\c$\\GitDevelopment\\express",
        "c:\\test with %\\path",
        "c:\\test with %25\\path",
        "c:\\test with %25\\c#code",
        "\\\\shares",
        "\\\\shares\\",
    ];
    let [
        drive,
        share,
        dollar,
        percent,
        escape,
        hash,
        bare_share,
        trailing_share,
    ] = paths.map(Uri::file);

    assert_eq!(drive.path(), "/c:/test/drive");
    assert_eq!(drive.to_string(), "file:///c%3A/test/drive");
    assert_components(&share, ["file", "shäres", "/path/c#/plugin.json", "", ""]);
    assert_eq!(
        share.to_string(),
        "file://sh%C3%A4res/path/c%23/plugin.json"
    );
    assert_eq!(dollar.path(), "/c$/GitDevelopment/express");
    assert_eq!(
        dollar.fs_path(),
        "\\\\localhost\\c$\\GitDevelopment\\express"
    );
    assert_eq!(
        dollar.to_string(),
        "file://localhost/c%24/GitDevelopment/express"
    );
    assert_eq!(percent.path(), "/c:/test with %/path");
    assert_eq!(percent.to_string(), "file:///c%3A/test%20with%20%25/path");
    assert_eq!(escape.path(), "/c:/test with %25/path");
    assert_eq!(escape.to_string(), "file:///c%3A/test%20with%20%2525/path");
    assert_eq!(hash.path(), "/c:/test with %25/c#code");
    assert_eq!(
        hash.to_string(),
        "file:///c%3A/test%20with%20%2525/c%23code"
    );
    assert_components(&bare_share, ["file", "shares", "/", "", ""]);
    assert_components(&trailing_share, ["file", "shares", "/", "", ""]);
}

#[test]
fn should_not_take_an_underscore_for_a_drive_letter() {
    let uri = parse("file:///_:/path");
    let fs_path = uri.fs_path();

    assert_eq!(
        fs_path,
        if cfg!(windows) {
            "\\_:\\path"
        } else {
            "/_:/path"
        }
    );
}

#[test]
fn should_take_a_file_uri_given_to_file_as_a_path() {
    let path = "file://path/to/file";
    let value = Uri::file(path);

    assert_components(&value, ["file", "", "/file://path/to/file", "", ""]);
}

#[test]
fn should_start_a_file_path_with_a_slash() {
    let path = "a.file";
    let value = Uri::file(path);

    assert_components(&value, ["file", "", "/a.file", "", ""]);
    assert_eq!(value.to_string(), "file:///a.file");
}

#[test]
fn should_keep_the_leading_slash_after_a_reparse() {
    let printed = Uri::file("a.file").to_string();
    let value = parse(&printed);

    assert_components(&value, ["file", "", "/a.file", "", ""]);
    assert_eq!(value.to_string(), "file:///a.file");
}

#[test]
fn should_print_a_uri_of_only_scheme_and_query() {
    let uri = parse("stuff:?qüery");
    let printed = uri.to_string();

    assert_eq!(printed, "stuff:?q%C3%BCery");
}

#[test]
fn should_print_percent_escapes_in_upper_case() {
    let uri = parse("file://sh%c3%a4res/path");
    let printed = uri.to_string();

    assert_eq!(printed, "file://sh%C3%A4res/path");
}

#[test]
fn should_print_a_windows_drive_letter_in_lower_case() {
    let uris = [
        parse("untitled:c:/Users/jrieken/Code/abc.txt"),
        parse("untitled:C:/Users/jrieken/Code/abc.txt"),
    ];
    let [lower, upper] = uris.map(|uri| uri.to_string());

    assert_eq!(lower, "untitled:c%3A/Users/jrieken/Code/abc.txt");
    assert_eq!(upper, "untitled:c%3A/Users/jrieken/Code/abc.txt");
}

#[test]
fn should_escape_all_the_bits_when_printing() {
    let value = Uri::file("/Users/jrieken/Code/_samples/18500/Mödel + Other Thîngß/model.js");
    let printed = value.to_string();

    assert_eq!(
        printed,
        "file:///Users/jrieken/Code/_samples/18500/M%C3%B6del%20%2B%20Other%20Th%C3%AEng%C3%9F/model.js"
    );
}

#[test]
fn should_not_encode_the_port_when_printing() {
    let uris = [
        parse("http://localhost:8080/far"),
        from("http", "löcalhost:8080", "/far", "", ""),
    ];
    let [ascii, international] = uris.map(|uri| uri.to_string());

    assert_eq!(ascii, "http://localhost:8080/far");
    assert_eq!(international, "http://l%C3%B6calhost:8080/far");
}

#[test]
fn should_keep_user_information_in_the_authority() {
    let values = [
        "http://foo:bar@localhost/far",
        "http://foo@localhost/far",
        "http://foo:bAr@localhost:8080/far",
        "http://foo@localhost:8080/far",
    ];
    let outcomes = values.map(|value| parse(value).to_string());

    for (value, printed) in values.iter().zip(&outcomes) {
        assert_eq!(printed, value);
    }
}

#[test]
fn should_encode_international_user_information() {
    let uri = from("http", "föö:bör@löcalhost:8080", "/far", "", "");
    let printed = uri.to_string();

    assert_eq!(
        printed,
        "http://f%C3%B6%C3%B6:b%C3%B6r@l%C3%B6calhost:8080/far"
    );
}

#[test]
fn should_turn_a_file_uri_into_a_file_path() {
    let windows = cfg!(windows);
    let cases = [
        (
            "file:///c:/alex.txt",
            if windows {
                "c:\\alex.txt"
            } else {
                "c:/alex.txt"
            },
        ),
        (
            "file:///c:/Source/Z%C3%BCrich%20or%20Zurich%20(%CB%88zj%CA%8A%C9%99r%C9%AAk,/Code/resources/app/plugins",
            if windows {
                "c:\\Source\\Zürich or Zurich (ˈzjʊərɪk,\\Code\\resources\\app\\plugins"
            } else {
                "c:/Source/Zürich or Zurich (ˈzjʊərɪk,/Code/resources/app/plugins"
            },
        ),
        (
            "file://monacotools/folder/isi.txt",
            if windows {
                "\\\\monacotools\\folder\\isi.txt"
            } else {
                "//monacotools/folder/isi.txt"
            },
        ),
        (
            "file://monacotools1/certificates/SSL/",
            if windows {
                "\\\\monacotools1\\certificates\\SSL\\"
            } else {
                "//monacotools1/certificates/SSL/"
            },
        ),
    ];
    let outcomes: Vec<_> = cases
        .iter()
        .map(|(input, _)| {
            let value = parse(input);
            let file = Uri::file(&value.fs_path());

            (value, file)
        })
        .collect();

    for ((input, expected), (value, file)) in cases.iter().zip(&outcomes) {
        assert_eq!(value.fs_path(), *expected, "{input}");
        assert_eq!(file.fs_path(), *expected, "{input}");
        assert_eq!(value.to_string(), file.to_string());
    }
}

#[test]
fn should_print_an_http_query_encoded_and_unencoded() {
    let sources = [
        "https://go.microsoft.com/fwlink/?LinkId=518008",
        "https://go.microsoft.com/fwlink/?LinkId=518008&foö&ké¥=üü",
    ];
    let [simple, international] = sources.map(parse);

    assert_eq!(simple.query(), "LinkId=518008");
    assert_eq!(
        simple.to_unencoded_string(),
        "https://go.microsoft.com/fwlink/?LinkId=518008"
    );
    assert_eq!(
        simple.to_string(),
        "https://go.microsoft.com/fwlink/?LinkId%3D518008"
    );
    assert_eq!(international.query(), "LinkId=518008&foö&ké¥=üü");
    assert_eq!(
        international.to_unencoded_string(),
        "https://go.microsoft.com/fwlink/?LinkId=518008&foö&ké¥=üü"
    );
    assert_eq!(
        international.to_string(),
        "https://go.microsoft.com/fwlink/?LinkId%3D518008%26fo%C3%B6%26k%C3%A9%C2%A5%3D%C3%BC%C3%BC"
    );
}

#[test]
fn should_keep_an_http_query_after_a_reparse() {
    let uris = [
        parse("https://go.microsoft.com/fwlink/?LinkId=518008"),
        parse("https://go.microsoft.com/fwlink/?LinkId=518008&foö&ké¥=üü"),
    ];
    let [simple, international] = uris.each_ref().map(|uri| parse(&uri.to_string()));

    assert_eq!(simple.query(), "LinkId=518008");
    assert_eq!(international.query(), uris[1].query());
}

#[test]
fn should_keep_an_encoded_hash_in_an_unencoded_query() {
    let uri = parse("https://twitter.com/search?src=typd&q=%23tag");
    let unencoded = uri.to_unencoded_string();

    assert_eq!(unencoded, "https://twitter.com/search?src=typd&q=%23tag");
}

#[test]
fn should_prefix_a_relative_file_path_with_a_slash() {
    let paths = ["/foo/bar", "foo/bar", "./foo/bar"];
    let [absolute, relative, dotted] = paths.map(Uri::file);

    assert_eq!(absolute.path(), "/foo/bar");
    assert_eq!(relative.path(), "/foo/bar");
    assert_eq!(dotted.path(), "/./foo/bar");
}

#[test]
fn should_parse_a_relative_file_uri_as_absolute() {
    let source = "file:foo/bar";
    let file = parse(source);

    assert_eq!(file.path(), "/foo/bar");
    assert_eq!(file.authority(), "");
    assert_eq!(file.to_string(), "file:///foo/bar");
}

#[test]
fn should_keep_a_relative_file_uri_absolute_after_a_reparse() {
    let uri = parse("file:foo/bar").to_string();
    let again = parse(&uri);

    assert_eq!(again.path(), "/foo/bar");
    assert_eq!(again.authority(), "");
}

#[test]
fn should_keep_a_query_after_a_hash_in_an_http_url() {
    let inputs = [
        "http://localhost:3000/#/foo?bar=baz",
        "http://localhost:3000/foo?bar=baz",
    ];
    let outcomes = inputs.map(|input| parse(input).to_unencoded_string());

    for (input, unencoded) in inputs.iter().zip(&outcomes) {
        assert_eq!(unencoded, input);
    }
}

#[test]
fn should_round_trip_malformed_percent_escapes_in_a_file_path() {
    let paths = ["/foo/%A0.txt", "/foo/%2e.txt"];
    let outcomes = paths.map(|path| {
        let uri = Uri::file(path);
        let again = parse(&uri.to_string());

        (uri, again)
    });

    for (uri, again) in &outcomes {
        assert_eq!(uri.scheme(), again.scheme());
        assert_eq!(uri.path(), again.path());
    }
}

#[test]
fn should_encode_a_lone_percent_sign() {
    let uris = [parse("file://some/%.txt"), parse("file://some/%A0.txt")];
    let [lone, escape] = uris.map(|uri| uri.to_string());

    assert_eq!(lone, "file://some/%25.txt");
    assert_eq!(escape, "file://some/%25A0.txt");
}

#[test]
fn should_serialize_as_its_string() {
    let uri = Uri::parse("file:///c:/My%20Project/main.src").expect("parses");
    let json = serde_json::to_string(&uri).expect("serializes");

    assert_eq!(json, "\"file:///c%3A/My%20Project/main.src\"");
}

#[test]
fn should_deserialize_from_its_string() {
    let uri = Uri::parse("file:///c:/My%20Project/main.src").expect("parses");
    let json = serde_json::to_string(&uri).expect("serializes");
    let back: Uri = serde_json::from_str(&json).expect("deserializes");

    assert_eq!(back, uri);
}

#[test]
fn should_reject_an_invalid_uri_on_deserialize() {
    let json = "\"file:////x\"";
    let result = serde_json::from_str::<Uri>(json);

    assert!(result.is_err());
}
