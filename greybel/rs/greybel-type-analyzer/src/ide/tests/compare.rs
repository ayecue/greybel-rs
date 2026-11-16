use super::NameComparison;

#[test]
fn should_count_a_swap_of_adjacent_characters_as_one_edit() {
    assert_eq!(NameComparison::edit_distance("rerurn", "return"), 1);
    assert_eq!(NameComparison::edit_distance("retrun", "return"), 1);
    assert_eq!(NameComparison::edit_distance("abc", "abc"), 0);
    assert_eq!(NameComparison::edit_distance("", "abc"), 3);
}

#[test]
fn should_match_the_closest_names_first() {
    let names =
        NameComparison::new("treeDirectry").closest(["treeDirectory", "deepScan", "treeDir"]);

    assert_eq!(names, ["treeDirectory"]);
}

#[test]
fn should_match_names_that_differ_only_in_case() {
    let names = NameComparison::new("infor").closest(["Info", "Infor", "inform"]);

    assert_eq!(names, ["Infor", "inform"]);
}

#[test]
fn should_skip_names_too_far_from_short_names() {
    let names = NameComparison::new("ab").closest(["xy", "abc", "ab"]);

    assert_eq!(names, ["abc"]);
}
