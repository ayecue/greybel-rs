//! The API reference (`greyscript-meta`) describes every native Greybel registers, with the native's own argument names
//! and defaults, so the editor's completions, signatures and the API site can't drift from what a script can call.

use greybel_interpreter::{Native, NativeDefault, ValueType};
use meta_utils::{ArgumentDefault, Container, Definition};

use crate::game::client::object::Kind;
use crate::game::{CLASSES, PLAYER_GLOBALS, global_name};

/// The tag of definitions only Greybel's interpreter has, such as `admin.so`'s classes.
const GREYBEL_ONLY: &str = "greybel only";

/// The meta type each value type's intrinsics are listed under.
fn value_type_name(kind: ValueType) -> &'static str {
    match kind {
        ValueType::String => "string",
        ValueType::Number => "number",
        ValueType::List => "list",
        ValueType::Map => "map",
        ValueType::Function => "function",
    }
}

/// The reference's type for `kind`'s class, which names some classes differently from their `classID`.
fn reference_type(kind: Kind) -> &'static str {
    match kind {
        Kind::FtpShell => "ftpShell",
        Kind::Metaxploit => "metaxploit",
        Kind::MetaLib => "metaLib",
        Kind::NetSession => "netSession",
        Kind::Crypto => "crypto",
        Kind::AptClient => "aptClient",
        Kind::Blockchain => "blockchain",
        Kind::SubWallet => "subWallet",
        Kind::MetaMail => "metaMail",
        Kind::SmartAppliance => "smartAppliance",
        Kind::TrafficNet => "trafficNet",
        _ => kind.class_id(),
    }
}

/// Whether `kind`'s class exists only in Greybel's interpreter.
fn is_greybel_only(kind: Kind) -> bool {
    matches!(kind, Kind::Admin | Kind::AdminNetwork | Kind::AdminComputer)
}

/// What differs between `native` and the meta `definition` of `owner.name`; `receiver` is the leading `self` argument
/// a detached method's global has in the reference.
fn differences(
    owner: &str,
    name: &str,
    native: &Native,
    definition: Option<&Definition>,
    receiver: bool,
) -> Vec<String> {
    let Some(function) = definition.and_then(Definition::as_function) else {
        return vec![format!("{owner}.{name}: missing")];
    };
    let mut expected: Vec<(&str, Option<String>)> = Vec::new();
    // A method's receiver is a parameter only when it has a default, as `to_int`'s.
    let has_receiver_param = native
        .params()
        .first()
        .is_some_and(|param| param.name() == "self");

    if receiver && native.is_method() && !has_receiver_param {
        expected.push(("self", None));
    }

    expected.extend(
        native
            .params()
            .iter()
            .filter(|param| receiver || param.name() != "self")
            .map(|param| {
                let default = match param.default() {
                    NativeDefault::Null => None,
                    NativeDefault::Number(number) => Some(number.to_string()),
                    NativeDefault::String(text) => Some(format!("{text:?}")),
                };

                (param.name(), default)
            }),
    );

    let actual: Vec<(&str, Option<String>)> = function
        .arguments()
        .iter()
        .map(|argument| {
            let default = argument.default().map(|default| match default {
                ArgumentDefault::Number(number) => number.to_string(),
                ArgumentDefault::String(text) => format!("{text:?}"),
            });

            (argument.label(), default)
        })
        .collect();

    if actual == expected {
        Vec::new()
    } else {
        vec![format!(
            "{owner}.{name}: native {expected:?}, reference {actual:?}"
        )]
    }
}

fn container() -> &'static Container {
    greyscript_meta::container()
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
