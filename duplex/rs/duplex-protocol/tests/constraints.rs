use duplex_protocol::Definition;

use crate::support::{SHOP, error_message, protocol_source};

#[test]
fn should_accept_a_consistent_definition() {
    let source = SHOP;

    let result = Definition::parse(source);

    assert!(result.is_ok(), "{result:?}");
}

#[test]
fn should_reject_a_failure_that_is_no_enumeration_or_union() {
    let failures = ["string", "S", "list<E>", "E?"];

    let messages = failures.map(|failure| {
        error_message(Definition::parse(&protocol_source(
            &format!("client C {{ message R {{}} -> Result<bool, {failure}> }} provider D {{}}"),
            "struct S {}\nenum E { A }",
        )))
    });

    assert!(
        messages
            .iter()
            .all(|message| message == "1:45: a response fails with a declared enumeration or union"),
        "{messages:?}"
    );
}

#[test]
fn should_accept_a_failure_of_a_union() {
    let source = "use core 1.0; protocol P = \"p\" { client C { message R {} -> Result<bool, U> } provider D {} }\nunion U { A(string) }";

    let result = Definition::parse(source);

    assert!(result.is_ok(), "{result:?}");
}

#[test]
fn should_reject_an_undeclared_type() {
    let source = "struct S { a: list<Missing> }";

    let message = error_message(Definition::parse(source));

    assert_eq!(message, "1:12: no type `Missing` is declared");
}

#[test]
fn should_reject_a_name_declared_twice() {
    let source = "struct S {}\nenum S { A }";

    let message = error_message(Definition::parse(source));

    assert_eq!(message, "2:1: `S` is declared twice");
}

#[test]
fn should_reject_a_type_named_like_a_generated_router() {
    let source =
        "use core 1.0; protocol P = \"p\" { client C {} provider D {} }\nstruct CRouter {}";

    let message = error_message(Definition::parse(source));

    assert_eq!(message, "1:34: `CRouter` is declared twice");
}

#[test]
fn should_reject_a_field_declared_twice_through_a_flattened_struct() {
    let source = "struct Inner { a: bool }\nstruct Outer { a: u8, flatten inner: Inner }";

    let message = error_message(Definition::parse(source));

    assert_eq!(message, "1:16: the field `a` is declared twice");
}

#[test]
fn should_reject_flattening_what_is_no_struct() {
    let source = "enum E { A }\nstruct S { flatten e: E }";

    let message = error_message(Definition::parse(source));

    assert_eq!(
        message,
        "2:12: the field `e` flattens a struct, which is neither optional nor nullable"
    );
}

#[test]
fn should_reject_a_type_containing_itself() {
    let source = "union Tree { Leaf, Node(Tree?) }";

    let message = error_message(Definition::parse(source));

    assert_eq!(
        message,
        "1:1: `Tree` contains itself; put it in a list or a record"
    );
}

#[test]
fn should_accept_a_type_containing_itself_in_a_list() {
    let source = "union Tree { Leaf, Node(list<Tree>) }";

    let result = Definition::parse(source);

    assert!(result.is_ok(), "{result:?}");
}

#[test]
fn should_reject_names_in_the_wrong_case() {
    let sources = ["struct S { newLine: bool }", "union U { lower }"];

    let messages = sources.map(|source| error_message(Definition::parse(source)));

    assert_eq!(
        messages,
        [
            "1:12: `newLine` is a field name in snake_case",
            "1:11: `lower` is a name in PascalCase",
        ]
    );
}

#[test]
fn should_reject_a_union_without_variants() {
    let source = "union U {}";

    let message = error_message(Definition::parse(source));

    assert_eq!(message, "1:1: the union `U` has variants");
}

#[test]
fn should_reject_values_given_to_some_members_only() {
    let source = "enum E { A = 1, B }";

    let message = error_message(Definition::parse(source));

    assert_eq!(
        message,
        "1:17: the enumeration `E` gives every member a value or none"
    );
}

#[test]
fn should_reject_a_value_given_twice() {
    let source = "enum E { A = 1, B = 1 }";

    let message = error_message(Definition::parse(source));

    assert_eq!(message, "1:17: the value 1 is given to another member");
}

#[test]
fn should_accept_defaults_of_their_fields_types() {
    let source = "const LIMIT: u16 = 300;\nconst NAME: string = \"n\";\nenum E { A = 1, B = 4 }\nstruct S { a: u16 = LIMIT, b: string = NAME, c: E = B, d: f32 = -1.5, e: char = \"x\", f: bool = false, g: list<u8>? = null, h: i8 = -128 }";

    let result = Definition::parse(source);

    assert!(result.is_ok(), "{result:?}");
}

#[test]
fn should_reject_a_default_of_another_type() {
    let sources = [
        "struct S { a: u8 = 256 }",
        "struct S { a: u8 = -1 }",
        "struct S { a: u8 = 1.5 }",
        "struct S { a: string = 1 }",
        "struct S { a: char = \"ab\" }",
        "struct S { a: E = C }\nenum E { A }",
        "struct S { a: list<u8> = 1 }",
        "struct S { a?: u8 = 1 }",
        "const A: u16 = 1;\nstruct S { a: u8 = A }",
    ];

    let messages = sources.map(|source| error_message(Definition::parse(source)));

    assert_eq!(
        messages,
        [
            "1:12: the default of `a` is out of the range of u8",
            "1:12: the default of `a` is out of the range of u8",
            "1:12: the default of `a` isn't an integer",
            "1:12: the default of `a` isn't a value of string",
            "1:12: the default of `a` isn't one character",
            "1:12: the default of `a` names no member of `E`",
            "1:12: the field `a` is of list<u8>, whose default can only be null",
            "1:12: the field `a` has a default, so it is neither optional nor flattened: it reads as its default",
            "2:12: the default of `a` names the constant `A`, of another type than u8",
        ]
    );
}

#[test]
fn should_reject_constants_that_break_their_rules() {
    let sources = [
        "const limit: u8 = 1;",
        "const A: u8 = 1;\nconst A: u8 = 2;",
        "const A: list<u8> = 1;",
        "const A: u8 = 1;\nconst B: u8 = A;",
    ];

    let messages = sources.map(|source| error_message(Definition::parse(source)));

    assert_eq!(
        messages,
        [
            "1:1: `limit` is a constant's name in CAPITALIZED_SNAKE_CASE",
            "2:1: the constant `A` is declared twice",
            "1:1: the constant `A` is of list<u8>; a constant is a boolean, character, text, number or member of an enumeration",
            "2:1: the constant `B` is a value, not another constant",
        ]
    );
}

#[test]
fn should_reject_a_flavour_declared_twice() {
    let source = "use core 1.0; protocol P = \"p\" { flavour stdio { health_check = off; } flavour stdio {} client C {} provider D {} }";

    let message = error_message(Definition::parse(source));

    assert_eq!(message, "1:72: the flavour of `stdio` is declared twice");
}
