use crate::support::type_expr::round_trip;

#[test]
fn should_parse_names_generics_and_unions() {
    let name = "string";
    let spaced = " map< string , list<shell> > | null ";
    let union = "list<string>|list<number>";

    let name = round_trip(name);
    let spaced = round_trip(spaced);
    let union = round_trip(union);

    assert_eq!(name.as_deref(), Some("string"));
    assert_eq!(spaced.as_deref(), Some("map<string,list<shell>>|null"));
    assert_eq!(union.as_deref(), Some("list<string>|list<number>"));
}

#[test]
fn should_reject_malformed_type_expressions() {
    let unclosed = "list<";
    let two_names = "a b";
    let empty = "";
    let nested = "list<".repeat(100_000);

    let unclosed = round_trip(unclosed);
    let two_names = round_trip(two_names);
    let empty = round_trip(empty);
    let nested = round_trip(&nested);

    assert_eq!(unclosed, None);
    assert_eq!(two_names, None);
    assert_eq!(empty, None);
    assert_eq!(nested, None);
}

#[test]
fn should_parse_function_types() {
    let arrow = "(value: number, string) => string|null";
    let written = "function(value: number): list<string>";
    let bare = "function()";

    let arrow = round_trip(arrow);
    let written = round_trip(written);
    let bare = round_trip(bare);

    assert_eq!(
        arrow.as_deref(),
        Some("function(value: number, string): string|null")
    );
    assert_eq!(
        written.as_deref(),
        Some("function(value: number): list<string>")
    );
    assert_eq!(bare.as_deref(), Some("function()"));
}

#[test]
fn should_parse_literal_tuple_and_guard_types() {
    let literals = "\"tcp\" | \"say \"\"hi\"\"\" | -1.5 | 22";
    let tuple = "[string, list<number>]";
    let guard = "value is Dog|null";

    let literals = round_trip(literals);
    let tuple = round_trip(tuple);
    let guard = round_trip(guard);

    assert_eq!(
        literals.as_deref(),
        Some("\"tcp\"|\"say \"\"hi\"\"\"|-1.5|22")
    );
    assert_eq!(tuple.as_deref(), Some("[string, list<number>]"));
    assert_eq!(guard.as_deref(), Some("value is Dog|null"));
}
