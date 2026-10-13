//! Computer-object intrinsics (`ComputerIntrinsics`), one file per method.

use std::time::Duration;

use greybel_interpreter::{Native, Value, Vm};

use crate::game::client::object::Access;
use crate::game::client::receiver;
use crate::game::client::receiver::receiver_for;
use crate::game::client::stopwatch;

mod active_net_card;
mod change_password;
mod close_program;
mod connect_ethernet;
mod connect_wifi;
mod create_folder;
mod create_group;
mod create_user;
mod delete_group;
mod delete_user;
mod file;
mod get_name;
mod get_ports;
mod groups;
mod is_network_active;
mod local_ip;
mod network_devices;
mod network_gateway;
mod public_ip;
mod reboot;
mod show_procs;
mod touch;
mod wifi_networks;

pub(crate) const METHODS: &[(&str, &Native)] = &[
    ("get_ports", &get_ports::get_ports::NATIVE),
    ("File", &file::file::NATIVE),
    ("create_folder", &create_folder::create_folder::NATIVE),
    (
        "is_network_active",
        &is_network_active::is_network_active::NATIVE,
    ),
    ("local_ip", &local_ip::local_ip::NATIVE),
    ("public_ip", &public_ip::public_ip::NATIVE),
    ("touch", &touch::touch::NATIVE),
    ("show_procs", &show_procs::show_procs::NATIVE),
    ("network_devices", &network_devices::network_devices::NATIVE),
    ("change_password", &change_password::change_password::NATIVE),
    ("create_user", &create_user::create_user::NATIVE),
    ("delete_user", &delete_user::delete_user::NATIVE),
    ("create_group", &create_group::create_group::NATIVE),
    ("delete_group", &delete_group::delete_group::NATIVE),
    ("groups", &groups::groups::NATIVE),
    ("close_program", &close_program::close_program::NATIVE),
    ("active_net_card", &active_net_card::active_net_card::NATIVE),
    ("network_gateway", &network_gateway::network_gateway::NATIVE),
    ("get_name", &get_name::get_name::NATIVE),
    ("reboot", &reboot::reboot::NATIVE),
    ("wifi_networks", &wifi_networks::wifi_networks::NATIVE),
    ("connect_wifi", &connect_wifi::connect_wifi::NATIVE),
    (
        "connect_ethernet",
        &connect_ethernet::connect_ethernet::NATIVE,
    ),
];

/// Methods available on FTP computer objects (`FtpComputerType`, `GreyInterpreter.FtpComputerType`).
pub(crate) const FTP_METHODS: &[(&str, &Native)] = &[
    ("File", &file::file::NATIVE),
    ("create_folder", &create_folder::create_folder::NATIVE),
    ("get_name", &get_name::get_name::NATIVE),
];

/// Returns computer object `this` when its type provides `method`.
pub(crate) fn computer(vm: &Vm, this: &Value, method: &str) -> Option<Access> {
    receiver_for::<receiver::Computer>(vm, this, method).cloned()
}

/// Returns computer object `this` and two string arguments, or `None` when an argument is not a string or `this` lacks `method`.
/// Checks the arguments before the receiver, as `ComputerIntrinsics` does.
fn computer_with_strings<'a>(
    vm: &Vm,
    this: &Value,
    method: &str,
    first: &'a Value,
    second: &'a Value,
) -> Option<(Access, &'a str, &'a str)> {
    let (first, second) = (first.as_str()?, second.as_str()?);

    Some((computer(vm, this, method)?, first, second))
}

/// Stopwatch tracking the program's last `connect_wifi` or `connect_ethernet` network switch (`HelperScript.GetSwitchConnWatch`).
struct SwitchConnectionWatch;

/// Time within which a second network switch must wait (`ComputerIntrinsics.connect_wifi`, `connect_ethernet`).
const SWITCH_COOLDOWN: Duration = Duration::from_secs(1);

/// Returns the VM time and whether the previous network switch is recent enough that this one must wait.
fn switch_wait(vm: &Vm) -> (Duration, bool) {
    let wait = stopwatch::elapsed_ms::<SwitchConnectionWatch>(vm)
        .is_some_and(|elapsed| elapsed < SWITCH_COOLDOWN.as_millis());

    (vm.clock().now(), wait)
}

/// Restarts the switch stopwatch at `now` once the server reports the switch started its wait.
fn restart_switch_watch(vm: &mut Vm, now: Duration, starts_wait: bool) {
    if starts_wait {
        stopwatch::restart_at::<SwitchConnectionWatch>(vm, now);
    }
}
