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
