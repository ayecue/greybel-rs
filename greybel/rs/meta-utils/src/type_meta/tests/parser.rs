#[path = "support/parser.rs"]
mod support;

use support::{nested, parsed};

use super::*;

#[test]
fn should_read_the_grammar() {
    let texts = [
        "number",
        "list",
        "map",
        " map < string , list < sub-string > > ",
        "list<map>",
        "map<list,map>",
    ];

    let [number, list, map, spaced, list_of_map, map_of_list_and_map] = texts.map(parsed);

    assert_eq!(number, "number");
    assert_eq!(list, "list<any>");
    assert_eq!(map, "map<any,any>");
    assert_eq!(spaced, "map<string,list<sub-string>>");
    assert_eq!(list_of_map, "list<map<any,any>>");
    assert_eq!(map_of_list_and_map, "map<list<any>,map<any,any>>");
}

#[test]
fn should_report_where_the_text_stops_fitting() {
    let texts = [
        "",
        "list<>",
        "list<string",
        "map<string>",
        "list>",
        "foo<bar>",
        "list<äöü>",
    ];

    let [
        empty,
        empty_list,
        unclosed,
        one_map_type,
        stray_close,
        generic_name,
        non_ascii,
    ] = texts.map(parse);

    assert_eq!(empty, Err(TypeParseError::ExpectedName { position: 0 }));
    assert_eq!(
        empty_list,
        Err(TypeParseError::ExpectedName { position: 5 })
    );
    assert_eq!(
        unclosed,
        Err(TypeParseError::MissingChar {
            character: '>',
            position: 11
        })
    );
    assert_eq!(
        one_map_type,
        Err(TypeParseError::MissingChar {
            character: ',',
            position: 10
        })
    );
    assert_eq!(
        stray_close,
        Err(TypeParseError::TrailingText { position: 4 })
    );
    assert_eq!(
        generic_name,
        Err(TypeParseError::TrailingText { position: 3 })
    );
    assert_eq!(non_ascii, Err(TypeParseError::ExpectedName { position: 5 }));
}

#[test]
fn should_limit_nesting() {
    let texts = [
        nested(MAX_TYPE_DEPTH),
        nested(MAX_TYPE_DEPTH + 1),
        "list<".repeat(100_000),
    ];

    let [at_limit, too_deep, unclosed] = texts.map(|text| parse(&text));

    assert!(at_limit.is_ok());
    assert_eq!(
        too_deep,
        Err(TypeParseError::DepthLimit {
            position: 5 * (MAX_TYPE_DEPTH + 1)
        })
    );
    assert!(unclosed.is_err());
}

#[test]
fn should_read_back_what_display_writes() {
    let any = || Box::new(TypeMeta::any());
    let metas = [
        TypeMeta::from_name("sub-string"),
        TypeMeta::List(Box::new(TypeMeta::Map {
            key: Box::new(TypeMeta::from_name("string")),
            value: Box::new(TypeMeta::List(any())),
        })),
        TypeMeta::Map {
            key: any(),
            value: Box::new(TypeMeta::Map {
                key: any(),
                value: any(),
            }),
        },
    ];

    let outcomes: Vec<_> = metas.iter().map(|meta| parse(&meta.to_string())).collect();

    for (parsed, meta) in outcomes.into_iter().zip(metas) {
        assert_eq!(parsed, Ok(meta));
    }
}
