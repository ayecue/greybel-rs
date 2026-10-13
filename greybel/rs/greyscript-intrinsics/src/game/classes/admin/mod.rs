//! Sandbox/test-only `admin.so` intrinsics for world construction and inspection; absent from Grey Hack.
//! Networks and computers use `adminNetwork` (`admin.network`) and `adminComputer` (`network.computer`) objects.

use greybel_interpreter::{Native, Value, Vm};

use crate::game::client::object;
use crate::game::client::object::{GameObject, Kind};

mod accept_mission;
mod buy_exploit;
mod create_ctf_account;
mod create_ctf_event;
mod create_debug_account;
mod create_guild;
mod create_mail_account;
mod create_network;
mod create_program;
mod deposit;
mod exploit_shop;
mod grant_coins;
mod network;
mod networks;
mod rate_ctf_event;
mod register_password;
mod seed;
mod set_date;
mod snapshot;
mod sudo;
mod tick;
mod traffic_accident;

/// Library filename for which `include_lib` returns the admin object.
pub(crate) const LIBRARY: &str = "admin.so";

pub(crate) const METHODS: &[(&str, &Native)] = &[
    ("seed", &seed::seed::NATIVE),
    ("snapshot", &snapshot::snapshot::NATIVE),
    ("tick", &tick::tick::NATIVE),
    (
        "traffic_accident",
        &traffic_accident::traffic_accident::NATIVE,
    ),
    ("set_date", &set_date::set_date::NATIVE),
    ("networks", &networks::networks::NATIVE),
    ("network", &network::network::NATIVE),
    ("create_network", &create_network::create_network::NATIVE),
    ("sudo", &sudo::sudo::NATIVE),
    ("create_program", &create_program::create_program::NATIVE),
    ("exploit_shop", &exploit_shop::exploit_shop::NATIVE),
    ("buy_exploit", &buy_exploit::buy_exploit::NATIVE),
    (
        "register_password",
        &register_password::register_password::NATIVE,
    ),
    (
        "create_mail_account",
        &create_mail_account::create_mail_account::NATIVE,
    ),
    (
        "create_ctf_account",
        &create_ctf_account::create_ctf_account::NATIVE,
    ),
    ("accept_mission", &accept_mission::accept_mission::NATIVE),
    (
        "create_debug_account",
        &create_debug_account::create_debug_account::NATIVE,
    ),
    ("deposit", &deposit::deposit::NATIVE),
    ("grant_coins", &grant_coins::grant_coins::NATIVE),
    (
        "create_ctf_event",
        &create_ctf_event::create_ctf_event::NATIVE,
    ),
    ("rate_ctf_event", &rate_ctf_event::rate_ctf_event::NATIVE),
    ("create_guild", &create_guild::create_guild::NATIVE),
];

pub(crate) fn create(vm: &mut Vm) -> Value {
    object::create(vm, Kind::Admin, GameObject::Admin)
}

pub(crate) fn is_admin(vm: &Vm, this: &Value, method: &str) -> bool {
    matches!(object::this(vm, this, method), Some(GameObject::Admin))
}

/// Converts a world value to a script number, with 53 bits of an integer like Grey Hack's doubles.
#[allow(clippy::cast_precision_loss)]
fn number(value: u64) -> f64 {
    value as f64
}
