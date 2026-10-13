use crate::support::parse;

#[test]
fn should_take_the_rest_of_the_line_as_the_statement() {
    let sources = ["x = 1 2", "x = fooand true", "return 1 2", "x = 1; y = 2"];

    let [two_values, glued_operator, two_returns, separated] = sources.map(parse);

    assert!(two_values.is_err());
    assert!(glued_operator.is_err());
    assert!(two_returns.is_err());
    assert!(separated.is_ok());
}

#[test]
fn should_not_start_a_statement_with_null() {
    let sources = ["null", "null != 1", "x = null", "true != 1"];

    let [null, null_comparison, null_value, true_comparison] = sources.map(parse);

    assert!(null.is_err());
    assert!(null_comparison.is_err());
    assert!(null_value.is_ok());
    assert!(true_comparison.is_ok());
}

#[test]
fn should_call_names_members_and_statement_indexes() {
    let sources = [
        "x = a(1)",
        "x = a.b(1)",
        "a[1](0)",
        "x = a(1).b(2)",
        "x = a (1)",
    ];

    let outcomes = sources.map(|source| (source, parse(source)));

    for (source, result) in outcomes {
        assert!(result.is_ok(), "{source}");
    }
}

#[test]
fn should_not_call_other_expressions() {
    let sources = [
        "x = a(1)(2)",
        "x = a[1](0)",
        "x = (a)(0)",
        "x = \"\"(0)",
        "print foo()()",
    ];

    let outcomes = sources.map(|source| (source, parse(source)));

    for (source, result) in outcomes {
        assert!(result.is_err(), "{source}");
    }
}

#[test]
fn should_not_take_new_after_new() {
    let sources = ["x = new new a", "x = new @a"];

    let [new_new, new_reference] = sources.map(parse);

    assert!(new_new.is_err());
    assert!(new_reference.is_ok());
}
