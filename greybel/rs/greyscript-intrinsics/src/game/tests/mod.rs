use std::collections::HashMap;
use std::ptr;

#[path = "support/mod.rs"]
mod support;

use greybel_interpreter::{Native, ValueType};
use support::{
    GREYBEL_ONLY, container, differences, is_greybel_only, reference_type, value_type_name,
};

use super::client::object::Kind;
use super::{CLASSES, PLAYER_GLOBALS, global_name};

/// Ensures no game global silently shadows another native.
#[test]
fn should_map_each_global_name_to_one_native() {
    let (classes, player_globals) = (CLASSES, PLAYER_GLOBALS);
    let globals: Vec<(&str, *const Native)> = classes
        .iter()
        .flat_map(|(table, overrides)| {
            table
                .iter()
                .map(move |&(key, native)| (global_name(key, overrides), native))
        })
        .chain(player_globals.iter().map(|&(name, native)| (name, native)))
        .map(|(name, native)| (name, std::ptr::from_ref::<Native>(native)))
        .collect();
    let mut seen: HashMap<&str, *const Native> = HashMap::new();

    for (name, native) in globals {
        if let Some(&existing) = seen.get(name) {
            assert!(
                ptr::eq(existing, native),
                "global `{name}` maps to two different natives",
            );
        }

        seen.insert(name, native);
    }
}

/// Grey Hack's interpreter finds a name it has no variable for among its intrinsics (`TAC.Context.GetVar`), and a
/// class's type map holds those intrinsics (`GreyInterpreter.ComputerType` and the like), so every method is also a
/// global, the same function: `File(host_computer(get_shell), "/")` is `get_shell.host_computer.File("/")`.
#[test]
fn should_give_every_game_class_method_as_the_same_global() {
    let globals: HashMap<*const Native, Vec<&str>> = CLASSES
        .iter()
        .flat_map(|(table, overrides)| {
            table.iter().map(move |&(key, native)| {
                (
                    std::ptr::from_ref::<Native>(native),
                    global_name(key, overrides),
                )
            })
        })
        .fold(HashMap::new(), |mut globals, (native, name)| {
            globals.entry(native).or_default().push(name);

            globals
        });

    let missing: Vec<String> = Kind::ALL
        .into_iter()
        .filter(|kind| !matches!(kind, Kind::Admin | Kind::AdminNetwork | Kind::AdminComputer))
        .flat_map(|kind| {
            kind.methods()
                .iter()
                .map(move |&(key, native)| (kind, key, native))
        })
        .filter(|&(_, key, native)| {
            let overrides = CLASSES
                .iter()
                .find(|(table, _)| table.iter().any(|&(_, other)| ptr::eq(other, native)))
                .map_or(&[][..], |(_, overrides)| overrides);

            !globals
                .get(&std::ptr::from_ref::<Native>(native))
                .is_some_and(|names| names.contains(&global_name(key, overrides)))
        })
        .map(|(kind, key, _)| format!("{}.{key}", kind.class_id()))
        .collect();

    assert_eq!(missing, Vec::<String>::new());
}

#[test]
fn should_describe_every_class_method_in_the_api_reference() {
    let mut problems = Vec::new();

    for kind in Kind::ALL {
        let owner = reference_type(kind);
        let Some(signature) = container().signature(owner) else {
            problems.push(format!("{owner}: no signature"));
            continue;
        };
        let definitions = signature.definitions();

        for &(name, native) in kind.methods() {
            problems.extend(differences(
                owner,
                name,
                native,
                definitions.get(name),
                false,
            ));
        }

        for (name, definition) in definitions {
            let registered = kind.methods().iter().any(|&(key, _)| key == name);

            if definition.is_function() && !registered {
                problems.push(format!("{owner}.{name}: not registered"));
            }

            if definition.has_tag(GREYBEL_ONLY) != is_greybel_only(kind) {
                problems.push(format!("{owner}.{name}: `{GREYBEL_ONLY}` tag is wrong"));
            }
        }
    }

    assert_eq!(problems, Vec::<String>::new());
}

#[test]
fn should_describe_every_global_in_the_api_reference() {
    let general = container()
        .signature("general")
        .expect("the general signature")
        .definitions();
    let mut problems = Vec::new();

    for (table, overrides) in CLASSES {
        for &(key, native) in *table {
            let name = global_name(key, overrides);

            problems.extend(differences(
                "general",
                name,
                native,
                general.get(name),
                true,
            ));
        }
    }

    for &(name, native) in crate::API.globals().iter().chain(PLAYER_GLOBALS) {
        problems.extend(differences(
            "general",
            name,
            native,
            general.get(name),
            true,
        ));
    }

    assert_eq!(problems, Vec::<String>::new());
}

#[test]
fn should_describe_every_value_intrinsic_in_the_api_reference() {
    let mut problems = Vec::new();

    for kind in ValueType::ALL {
        let owner = value_type_name(kind);
        let Some(signature) = container().signature(owner) else {
            if !crate::API.intrinsics(kind).is_empty() {
                problems.push(format!("{owner}: no signature"));
            }

            continue;
        };

        for &(name, native) in crate::API.intrinsics(kind) {
            problems.extend(differences(
                owner,
                name,
                native,
                signature.definitions().get(name),
                false,
            ));
        }
    }

    assert_eq!(problems, Vec::<String>::new());
}
