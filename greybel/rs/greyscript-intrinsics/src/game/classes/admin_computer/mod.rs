//! Sandbox/test `adminComputer` objects obtained through `admin.so` (`admin.network(public_ip).computer(lan_ip)`).

use greybel_interpreter::{Native, Value, Vm};

use crate::game::client::object;
use crate::game::client::object::{ComputerAddress, GameObject, Kind};
use crate::game::client::receiver::{self, receiver_for};

mod access_cctv;
mod add_net_card;
mod add_npc;
mod add_smart_appliance;
mod install_smartphone_adb;
mod login;
mod npc_login;
mod service_records;
mod set_library_version;
mod set_port_library_version;
mod shell;
mod update_service_record;

pub(crate) const METHODS: &[(&str, &Native)] = &[
    ("shell", &shell::shell::NATIVE),
    ("login", &login::login::NATIVE),
    ("add_npc", &add_npc::add_npc::NATIVE),
    ("npc_login", &npc_login::npc_login::NATIVE),
    ("add_net_card", &add_net_card::add_net_card::NATIVE),
    (
        "add_smart_appliance",
        &add_smart_appliance::add_smart_appliance::NATIVE,
    ),
    (
        "set_library_version",
        &set_library_version::set_library_version::NATIVE,
    ),
    (
        "set_port_library_version",
        &set_port_library_version::set_port_library_version::NATIVE,
    ),
    ("service_records", &service_records::service_records::NATIVE),
    (
        "update_service_record",
        &update_service_record::update_service_record::NATIVE,
    ),
    ("access_cctv", &access_cctv::access_cctv::NATIVE),
    (
        "install_smartphone_adb",
        &install_smartphone_adb::install_smartphone_adb::NATIVE,
    ),
];

pub(crate) fn create(vm: &mut Vm, address: ComputerAddress) -> Value {
    object::create(vm, Kind::AdminComputer, GameObject::AdminComputer(address))
}

/// Returns the represented computer when `this` is an `adminComputer` providing `method`.
fn address(vm: &Vm, this: &Value, method: &str) -> Option<ComputerAddress> {
    receiver_for::<receiver::AdminComputer>(vm, this, method).cloned()
}
