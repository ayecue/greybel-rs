#![allow(dead_code)]

pub mod operations;

use vscode_uri::{Uri, UriChange, UriComponentContainer};

pub fn parse(value: &str) -> Uri {
    Uri::parse(value).unwrap_or_else(|error| panic!("{value}: {error}"))
}

pub fn from(scheme: &str, authority: &str, path: &str, query: &str, fragment: &str) -> Uri {
    Uri::from_components(UriComponentContainer {
        scheme: scheme.to_owned(),
        authority: authority.to_owned(),
        path: path.to_owned(),
        query: query.to_owned(),
        fragment: fragment.to_owned(),
    })
    .expect("valid components")
}

pub fn change(scheme: Option<&str>, authority: Option<&str>, path: Option<&str>) -> UriChange {
    UriChange {
        scheme: scheme.map(str::to_owned),
        authority: authority.map(str::to_owned),
        path: path.map(str::to_owned),
        ..UriChange::default()
    }
}

pub fn assert_components(uri: &Uri, [scheme, authority, path, query, fragment]: [&str; 5]) {
    assert_eq!(uri.scheme(), scheme);
    assert_eq!(uri.authority(), authority);
    assert_eq!(uri.path(), path);
    assert_eq!(uri.query(), query);
    assert_eq!(uri.fragment(), fragment);
}
