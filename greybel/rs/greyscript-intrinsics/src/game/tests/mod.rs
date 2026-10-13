mod api_reference;

use std::collections::HashMap;
use std::ptr;

use greybel_interpreter::Native;

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
        .map(|(name, native)| (name, native as *const Native))
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
            table
                .iter()
                .map(move |&(key, native)| (native as *const Native, global_name(key, overrides)))
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
                .get(&(native as *const Native))
                .is_some_and(|names| names.contains(&global_name(key, overrides)))
        })
        .map(|(kind, key, _)| format!("{}.{key}", kind.class_id()))
        .collect();

    assert_eq!(missing, Vec::<String>::new());
}
