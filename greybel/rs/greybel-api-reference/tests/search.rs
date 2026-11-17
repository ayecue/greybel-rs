//! Tests reference search by type and member name, including excluded-tag filtering.

use crate::support::Harness;
use crate::support::search::path;

#[test]
fn should_match_members_by_type_and_name_ignoring_case() {
    let mut harness = Harness::new();

    let matches = harness.search("SHELL.HOST", &[]);

    assert!(matches.types.is_empty());
    assert_eq!(
        matches.members,
        [
            path("ftpShell", "host_computer"),
            path("shell", "host_computer")
        ]
    );
}

#[test]
fn should_match_a_type_and_all_its_members_by_its_name() {
    let mut harness = Harness::new();

    let matches = harness.search("ftpShell", &[]);

    assert_eq!(matches.types, ["ftpShell"]);
    assert!(matches.members.contains(&path("ftpShell", "host_computer")));
    assert!(
        matches
            .members
            .iter()
            .all(|member| member.owner == "ftpShell")
    );
}

#[test]
fn should_leave_out_members_with_an_excluded_tag() {
    let mut harness = Harness::new();

    let all = harness.search("", &[]);
    let without_methods = harness.search("", &["method"]);

    assert!(without_methods.types.contains(&"general".to_owned()));
    assert!(all.members.contains(&path("shell", "host_computer")));
    assert!(
        !without_methods
            .members
            .contains(&path("shell", "host_computer"))
    );
    assert!(
        without_methods
            .members
            .contains(&path("general", "get_shell"))
    );
}

#[test]
fn should_take_a_query_as_text() {
    let mut harness = Harness::new();

    let matches = harness.search("(", &[]);

    assert!(matches.types.is_empty() && matches.members.is_empty());
}

#[test]
fn should_leave_out_a_type_whose_members_all_have_an_excluded_tag() {
    let mut harness = Harness::new();

    let all = harness.search("", &[]);
    let without_greybel = harness.search("", &["greybel"]);

    for owner in ["admin", "adminComputer", "adminNetwork"] {
        assert!(all.types.contains(&owner.to_owned()));
        assert!(!without_greybel.types.contains(&owner.to_owned()));
        assert!(
            without_greybel
                .members
                .iter()
                .all(|member| member.owner != owner)
        );
    }

    assert!(without_greybel.types.contains(&"shell".to_owned()));
}
