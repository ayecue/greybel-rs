use crate::mesh::test_support::character;

#[test]
fn should_count_a_no_break_space_as_a_separator() {
    let space = character('\u{a0}');

    let separator = space.is_separator();

    assert!(separator);
}

#[test]
fn should_not_count_a_tab_as_a_separator_but_as_white_space() {
    let tab = character('\t');

    let classes = (tab.is_separator(), tab.is_white_space());

    assert_eq!(classes, (false, true));
}

#[test]
fn should_not_count_a_zero_width_space_as_white_space() {
    let zero_width = character('\u{200b}');

    let white_space = zero_width.is_white_space();

    assert!(!white_space);
}
