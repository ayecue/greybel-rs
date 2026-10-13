//! Router address, settings, port, and device intrinsics (`RouterIntrinsics`), one file per method.

use greybel_interpreter::{Native, Value, Vm};

use crate::error::IntrinsicError;
use crate::game::client::object::{GameObject, Kind};
use crate::game::client::receiver::receiver;
use crate::game::client::{object, receiver};
use crate::protocol::ObjectId;

mod bssid_name;
mod device_ports;
mod devices_lan_ip;
mod essid_name;
mod firewall_rules;
mod kernel_version;
mod local_ip;
mod net_speed;
mod ping_port;
mod port_info;
mod public_ip;
mod used_ports;

pub(crate) const METHODS: &[(&str, &Native)] = &[
    ("public_ip", &public_ip::public_ip::NATIVE),
    ("local_ip", &local_ip::local_ip::NATIVE),
    ("ping_port", &ping_port::ping_port::NATIVE),
    ("port_info", &port_info::port_info::NATIVE),
    ("used_ports", &used_ports::used_ports::NATIVE),
    ("device_ports", &device_ports::device_ports::NATIVE),
    ("devices_lan_ip", &devices_lan_ip::devices_lan_ip::NATIVE),
    ("essid_name", &essid_name::essid_name::NATIVE),
    ("bssid_name", &bssid_name::bssid_name::NATIVE),
    ("kernel_version", &kernel_version::kernel_version::NATIVE),
    ("net_speed", &net_speed::net_speed::NATIVE),
    ("firewall_rules", &firewall_rules::firewall_rules::NATIVE),
];

/// Returns the server identity of router object `this`.
fn router_id(vm: &Vm, this: &Value) -> Option<ObjectId> {
    receiver::<receiver::Router>(vm, this).map(|router| router.reference())
}

/// Constructs the router object for server-issued `id`.
pub(crate) fn create(vm: &mut Vm, id: ObjectId) -> Result<Value, IntrinsicError> {
    object::create_of(vm, Kind::Router, GameObject::Router, id)
}
