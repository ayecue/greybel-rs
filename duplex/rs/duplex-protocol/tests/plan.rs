use duplex_protocol::{
    Encoding, FieldValue, Length, MigrationError, MigrationPlan, NumberKind, PayloadPlan, Plan,
    Presence, Range, Role, StringRequirementContainer, TypeContent,
};

use crate::support::plan::{OLDER, bit, struct_fields_plan};
use crate::support::{SHOP, plan, version_history};

#[test]
fn should_lay_out_booleans_and_presences_in_the_flags() {
    let versions = version_history(&[(1, 0, SHOP)]).expect("the shop version is accepted");

    let plan = plan(&versions);

    let order = &plan
        .protocol
        .as_ref()
        .expect("the shop plan has a protocol")
        .messages[0]
        .body;
    let fields: Vec<_> = order
        .fields()
        .map(|field| (field.presence, field.value.clone()))
        .collect();

    assert_eq!(
        fields,
        [
            (
                Presence::default(),
                FieldValue::Value(Encoding::String(&StringRequirementContainer::default()))
            ),
            (
                Presence::default(),
                FieldValue::Value(Encoding::Number(NumberKind::U32, &Range::default()))
            ),
            (
                Presence {
                    present: Some(bit(0, 1)),
                    non_null: None
                },
                FieldValue::Flag(bit(0, 2))
            ),
            (
                Presence {
                    present: None,
                    non_null: Some(bit(0, 4))
                },
                FieldValue::Value(Encoding::String(&StringRequirementContainer::default()))
            ),
        ]
    );
    assert_eq!(order.blocks[0].flags, [0b111]);
}

#[test]
fn should_continue_the_flags_in_another_byte() {
    let versions = version_history(&[(
        1,
        0,
        "struct S { a: bool, b: bool, c: bool, d: bool, e: bool, f: bool, g: bool, h: bool?, i: u8 }",
    )])
    .expect("the test version is accepted");

    let plan = plan(&versions);

    let fields = struct_fields_plan(&plan, 0);

    assert_eq!(
        fields.blocks[0].fields[7].presence.non_null,
        Some(bit(0, 0x80))
    );
    assert_eq!(
        fields.blocks[0].fields[7].value,
        FieldValue::Flag(bit(1, 1))
    );
    assert_eq!(fields.blocks[0].flags, [0xff, 1]);
}

#[test]
fn should_lay_out_no_flags_without_booleans_or_presences() {
    let versions = version_history(&[(1, 0, "struct S { a: string, b: list<bool> }")])
        .expect("the test version is accepted");

    let plan = plan(&versions);

    assert_eq!(
        struct_fields_plan(&plan, 0).blocks[0].flags,
        Vec::<u8>::new()
    );
}

#[test]
fn should_resolve_what_declared_types_are() {
    let versions = version_history(&[(
        1,
        0,
        "struct S { u: U, e: list<E?>, s: record<S> }\nunion U { A, B(E) }\nenum E { X }",
    )])
    .expect("the test version is accepted");

    let plan = plan(&versions);

    let tys: Vec<_> = struct_fields_plan(&plan, 0)
        .fields()
        .map(|field| field.ty.clone())
        .collect();

    assert_eq!(
        tys,
        [
            Encoding::Union("U"),
            Encoding::List(
                Box::new(Encoding::Nullable(Box::new(Encoding::Enumeration("E")))),
                Length::ANY
            ),
            Encoding::Record(Box::new(Encoding::Struct("S")), Length::ANY),
        ]
    );

    let TypeContent::Union(variants) = &plan.types[1].content else {
        panic!("a union");
    };

    assert_eq!(
        variants
            .iter()
            .map(|variant| (variant.index, variant.payload.clone()))
            .collect::<Vec<_>>(),
        [
            (0, PayloadPlan::Unit),
            (1, PayloadPlan::Value(Encoding::Enumeration("E")))
        ]
    );
}

#[test]
fn should_plan_a_flattened_struct_in_its_place() {
    let versions = version_history(&[(
        1,
        0,
        "struct Outer { first: bool, flatten inner: Inner, last: bool }\nstruct Inner { flag: bool, text: string }",
    )])
    .expect("the test version is accepted");

    let plan = plan(&versions);

    let outer = struct_fields_plan(&plan, 0);
    let FieldValue::Flatten { name, fields } = &outer.blocks[0].fields[1].value else {
        panic!("a flattened struct");
    };

    assert_eq!(*name, "Inner");
    assert_eq!(fields.blocks[0].flags, [1]);
    assert_eq!(outer.blocks[0].flags, [0b11]);

    let properties: Vec<&str> = outer
        .properties()
        .iter()
        .map(|field| field.field.name.as_str())
        .collect();

    assert_eq!(properties, ["first", "flag", "text", "last"]);
}

#[test]
fn should_plan_the_messages_a_role_handles() {
    let versions = version_history(&[(1, 0, SHOP)]).expect("the shop version is accepted");
    let plan = plan(&versions)
        .protocol
        .expect("the shop plan has a protocol");

    let handled = [Role::Client, Role::Provider].map(|role| {
        let plan = plan.role(role);

        (
            plan.local().name.clone(),
            plan.inbound_messages()
                .map(|message| message.message.name.clone())
                .collect::<Vec<_>>(),
        )
    });

    assert_eq!(
        handled,
        [
            ("Customer".to_owned(), vec!["Confirm".to_owned()]),
            (
                "Counter".to_owned(),
                vec!["Order".to_owned(), "Cancel".to_owned()]
            ),
        ]
    );
}

#[test]
fn should_plan_no_protocol_for_a_definition_of_types() {
    let versions = version_history(&[(1, 0, "struct S {}")]).expect("the test version is accepted");

    let plan = plan(&versions);

    assert!(plan.protocol.is_none());
}

#[test]
fn should_write_members_as_their_values_else_their_positions() {
    let versions = version_history(&[(1, 0, "enum E { A, B }\nenum F { A = 7, B = 3 }")])
        .expect("the test version is accepted");

    let plan = plan(&versions);

    let values: Vec<Vec<u32>> = plan
        .types
        .iter()
        .map(|plan| match &plan.content {
            TypeContent::Enumeration(members) => {
                members.iter().map(|member| member.value).collect()
            }
            _ => panic!("an enumeration"),
        })
        .collect();

    assert_eq!(values, [vec![0, 1], vec![7, 3]]);
}

#[test]
fn should_pair_a_message_by_the_name_it_renames() {
    let versions = version_history(&[
        (1, 0, OLDER),
        (
            2,
            0,
            "use core 1.0; protocol P = \"p\" { open Login; client C {} provider D { message Total renames Sum { a: u8 } -> u8 } } struct Login { user: string }",
        ),
    ])
    .expect("the test versions are accepted");
    let (older, newer) = (
        Plan::of(versions.major(1).expect("the versions have major 1")),
        Plan::of(versions.major(2).expect("the versions have major 2")),
    );

    let migration =
        MigrationPlan::of(&older, &newer).expect("the matching handshakes allow a migration plan");

    let paired: Vec<_> = migration
        .messages
        .iter()
        .map(|message| {
            (
                message.older.message.name.as_str(),
                message.newer.message.name.as_str(),
            )
        })
        .collect();

    assert_eq!(paired, [("Sum", "Total")]);
    assert!(migration.messages[0].message.by_default);
}

#[test]
fn should_refuse_to_migrate_between_majors_whose_handshakes_differ() {
    let versions = version_history(&[
        (1, 0, OLDER),
        (
            2,
            0,
            "use core 1.0; protocol P = \"p\" { open Login; client C {} provider D { message Sum { a: u8 } -> u8 } } struct Login { user: string, token?: string }",
        ),
    ])
    .expect("the test versions are accepted");
    let (older, newer) = (
        Plan::of(versions.major(1).expect("the versions have major 1")),
        Plan::of(versions.major(2).expect("the versions have major 2")),
    );

    let migration = MigrationPlan::of(&older, &newer);

    assert_eq!(
        migration.err(),
        Some(MigrationError::HandshakeMismatch("open"))
    );
}
