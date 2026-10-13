use greybel_type_analyzer::*;

use crate::support::doc::written;

#[test]
fn should_read_tags_and_text() {
    let lines = [
        " Hello world",
        " I am **bold**",
        " @param {string} test - The title of the book.",
        " @params {string|number} abc - The author",
        "   of the book.",
        " @return {crypto} - Some info",
    ];

    let doc = DocComment::parse(&lines);

    assert_eq!(
        doc.description.as_deref(),
        Some("Hello world\nI am **bold**")
    );
    assert_eq!(doc.params.len(), 2);
    assert_eq!(written(&doc.params[1].ty).as_deref(), Some("string|number"));
    assert_eq!(
        doc.params[1].description.as_deref(),
        Some("The author\nof the book.")
    );
    assert_eq!(written(&doc.returns).as_deref(), Some("crypto"));
    assert_eq!(doc.returns_description.as_deref(), Some("Some info"));
    assert!(doc.invalid_tags.is_empty());
}

#[test]
fn should_keep_unknown_tags_in_the_description() {
    let lines = [
        " @public",
        " @description Hello world",
        " @example print \"test\"",
        " Hello world",
    ];

    let doc = DocComment::parse(&lines);

    assert_eq!(doc.description.as_deref(), Some("@public\nHello world"));
    assert_eq!(doc.examples, ["print \"test\"", "Hello world"]);
    assert!(doc.returns.is_none());
}

#[test]
fn should_read_type_declarations() {
    let lines = [
        " @vtype myCustomType",
        " @extends myOtherType",
        " @function foo",
        "   @params {string} bar",
        "   @returns {string}",
        "   @description This is a custom type with a function",
        "   @example Example usage",
        " @property {number} myProperty",
        " @property {readonly list} nested.items - the items",
    ];

    let doc = DocComment::parse(&lines);

    let [ty] = doc.types.as_slice() else {
        panic!("one type: {doc:?}");
    };

    assert!(ty.virtual_type);
    assert_eq!(ty.name, "myCustomType");
    assert_eq!(written(&ty.extends).as_deref(), Some("myOtherType"));

    let [function] = ty.functions.as_slice() else {
        panic!("one function");
    };

    assert_eq!(function.name, "foo");
    assert_eq!(function.params[0].name, "bar");
    assert_eq!(written(&function.returns).as_deref(), Some("string"));
    assert_eq!(
        function.description.as_deref(),
        Some("This is a custom type with a function")
    );
    assert_eq!(function.examples, ["Example usage"]);
    assert_eq!(ty.properties[0].path, ["myProperty"]);
    assert_eq!(ty.properties[1].path, ["nested", "items"]);
    assert_eq!(ty.properties[1].ty.to_string(), "list");
    assert_eq!(ty.properties[1].description.as_deref(), Some("the items"));
    assert!(doc.params.is_empty() && doc.invalid_tags.is_empty());
}

#[test]
fn should_read_type_parameters() {
    let lines = [" @type Box<T>", " @property {T} value"];

    let doc = DocComment::parse(&lines);

    assert_eq!(doc.types[0].type_parameters, ["T"]);
}

#[test]
fn should_read_templates_and_their_bounds() {
    let lines = [" @template TItem extends string", " @param {TItem} value"];

    let doc = DocComment::parse(&lines);

    assert_eq!(doc.templates[0].name, "TItem");
    assert_eq!(written(&doc.templates[0].bound).as_deref(), Some("string"));
}

#[test]
fn should_read_a_braced_type_tag_as_a_define() {
    let lines = [" @type {list<string>}"];

    let doc = DocComment::parse(&lines);

    assert_eq!(written(&doc.defined_type).as_deref(), Some("list<string>"));
    assert!(doc.types.is_empty());
}

#[test]
fn should_report_tags_it_cannot_read() {
    let lines = [
        " @param {list<} broken",
        " @property {number} orphan",
        " @template T = string",
        " @vtype",
    ];

    let doc = DocComment::parse(&lines);

    let lines: Vec<usize> = doc.invalid_tags.iter().map(|tag| tag.line).collect();

    assert_eq!(lines, [0, 1, 2, 3]);
}
