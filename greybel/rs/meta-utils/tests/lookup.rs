//! Tests inherited, ambiguous, and post-mutation lookups.

use meta_utils::{BaseType, Container, TypeLookup};

use crate::support::{
    add, inheriting_container, origin, other_described, thing_excluded, thing_included_again,
    thing_with_size, thing_without_size,
};

#[test]
fn should_list_own_members_then_inherited() {
    let container = inheriting_container();

    let members: Vec<(&str, &str, Option<&str>)> = container
        .members("thing", "en")
        .map(|member| (member.name(), member.origin(), member.description()))
        .collect();

    assert_eq!(
        members,
        [
            ("size", "thing", None),
            ("len", "thing", None),
            ("keys", "map", Some("The keys.")),
        ]
    );
}

#[test]
fn should_list_members_inherited_through_a_hidden_parent() {
    let container = inheriting_container();

    let names: Vec<&str> = container
        .members("child", "en")
        .map(|member| member.name())
        .collect();

    assert_eq!(names, ["own", "len", "keys"]);
}

#[test]
fn should_list_no_members_of_an_unknown_type() {
    let container = inheriting_container();

    let count = container.members("nothing", "en").count();

    assert_eq!(count, 0);
}

#[test]
fn should_prefer_any_among_several_origins() {
    let mut container = Container::new();

    add(&mut container, "any", None, &["size"]);
    add(&mut container, "a", Some("map"), &["size", "len"]);
    add(&mut container, "b", Some("map"), &["size", "len"]);
    add(&mut container, "c", Some("any"), &[]);

    let lookups: [(&[&str], &str); 5] = [
        (&["b"], "size"),
        (&["a", "b"], "size"),
        (&["a", "c"], "size"),
        (&["b", "a"], "len"),
        (&["b", "b"], "size"),
    ];

    let [b, a_b, a_c, b_a_len, b_b] =
        lookups.map(|(types, name)| origin(container.definition(types, name, "en")));

    assert_eq!(b, Some("b"));
    assert_eq!(a_b, Some("any"));
    assert_eq!(a_c, Some("any"));
    assert_eq!(b_a_len, Some("b"));
    assert_eq!(b_b, Some("b"));
}

#[test]
fn should_find_no_definition_the_type_does_not_have() {
    let container = thing_without_size();

    let definition = container.definition(&["thing"], "size", "en");

    assert!(definition.is_none());
}

#[test]
fn should_find_a_definition_added_by_a_later_signature() {
    let container = thing_with_size();

    let found = origin(container.definition(&["thing"], "size", "en"));

    assert_eq!(found, Some("thing"));
}

#[test]
fn should_search_a_definition_added_by_a_later_signature() {
    let container = thing_with_size();

    let found = container.search_definitions(&["any"], "size", "en");

    assert_eq!(found.len(), 1);
}

#[test]
fn should_not_search_a_type_excluded_from_search() {
    let container = thing_excluded();

    let found = container.search_definitions(&["any"], "size", "en");

    assert!(found.is_empty());
}

#[test]
fn should_find_no_definition_on_a_type_excluded_from_search() {
    let container = thing_excluded();

    let definition = container.definition(&["thing"], "size", "en");

    assert!(definition.is_none());
}

#[test]
fn should_search_a_type_included_in_search_again() {
    let container = thing_included_again();

    let found = container.search_definitions(&["any"], "size", "en");

    assert_eq!(found.len(), 1);
}

#[test]
fn should_list_no_members_of_a_type_with_only_descriptions() {
    let container = other_described();

    let count = container.members("other", "en").count();

    assert_eq!(count, 0);
}

#[test]
fn should_keep_the_members_of_a_type_after_other_changes() {
    let container = other_described();

    let count = container.members("thing", "en").count();

    assert_eq!(count, 1);
}

#[test]
fn should_resolve_type_names() {
    let mut container = Container::new();

    add(&mut container, "thing", Some("map"), &[]);

    let names = ["string", "list", "null", "thing", "nothing"];

    let [string, list, null, thing, nothing] = names.map(|name| container.lookup_type(name));

    assert!(matches!(string, TypeLookup::Base(BaseType::String)));
    assert!(matches!(list, TypeLookup::Base(BaseType::List)));
    assert!(matches!(null, TypeLookup::Null));
    assert!(matches!(
        thing,
        TypeLookup::Class(signature) if signature.type_name() == "thing"
    ));
    assert!(matches!(nothing, TypeLookup::Unknown));
}

#[test]
fn should_find_definitions_by_id() {
    let mut container = Container::new();

    add(&mut container, "string", None, &["len"]);
    add(&mut container, "list", None, &["len", "push"]);

    let ids = ["len", "nothing"];

    let [len, nothing] = ids.map(|id| {
        container
            .definitions_by_id(id, "en")
            .map(|found| (found.origin(), found.name()))
            .collect::<Vec<_>>()
    });

    assert_eq!(len, [("string", "len"), ("list", "len")]);
    assert_eq!(nothing.len(), 0);
}
