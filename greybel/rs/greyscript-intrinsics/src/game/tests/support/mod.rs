use greybel_interpreter::{Native, NativeDefault, ValueType};
use meta_utils::{ArgumentDefault, Container, Definition};

use crate::game::client::object::Kind;

/// The tag of definitions only Greybel's interpreter has, such as `admin.so`'s classes.
pub(super) const GREYBEL_ONLY: &str = "greybel only";

/// The meta type each value type's intrinsics are listed under.
pub(super) fn value_type_name(kind: ValueType) -> &'static str {
    match kind {
        ValueType::String => "string",
        ValueType::Number => "number",
        ValueType::List => "list",
        ValueType::Map => "map",
        ValueType::Function => "function",
    }
}

/// The reference's type for `kind`'s class, which names some classes differently from their `classID`.
pub(super) fn reference_type(kind: Kind) -> &'static str {
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
pub(super) fn is_greybel_only(kind: Kind) -> bool {
    matches!(kind, Kind::Admin | Kind::AdminNetwork | Kind::AdminComputer)
}

/// What differs between `native` and the meta `definition` of `owner.name`; `receiver` is the leading `self` argument
/// a detached method's global has in the reference.
pub(super) fn differences(
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

pub(super) fn container() -> &'static Container {
    greyscript_meta::container()
}
