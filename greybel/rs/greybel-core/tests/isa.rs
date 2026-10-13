use crate::support::parse;

#[test]
fn should_take_one_operand_per_side_of_isa() {
    let sources = [
        "x = a isa b isa c",
        "if a isa b isa c then y = 1",
        "x = [a isa b isa c]",
    ];

    let [assignment, condition, list] = sources.map(parse);

    assert!(assignment.is_err());
    assert!(condition.is_err());
    assert!(list.is_err());
}

#[test]
fn should_combine_isa_with_other_levels() {
    let sources = [
        "x = (a isa b) isa c",
        "x = a isa b < c",
        "x = a isa b and c isa d",
        "x = not a isa b",
    ];

    let [parenthesized, comparison, and, not] = sources.map(parse);

    assert!(parenthesized.is_ok());
    assert!(comparison.is_ok());
    assert!(and.is_ok());
    assert!(not.is_ok());
}
