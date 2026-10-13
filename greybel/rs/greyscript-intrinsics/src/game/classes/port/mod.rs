//! Port-object intrinsics (`PortIntrinsics`), one file per method.

use greybel_interpreter::{Native, Value, Vm};

use crate::error::IntrinsicError;
use crate::game::client::object::{GameObject, Kind};
use crate::game::client::receiver::receiver_or_crash;
use crate::game::client::{object, receiver};
use crate::protocol::Port;

mod get_lan_ip;
mod is_closed;
mod port_number;

pub(crate) const METHODS: &[(&str, &Native)] = &[
    ("get_lan_ip", &get_lan_ip::get_lan_ip::NATIVE),
    ("is_closed", &is_closed::is_closed::NATIVE),
    ("port_number", &port_number::port_number::NATIVE),
];

fn port<'v>(vm: &'v Vm, this: &Value) -> Result<Option<&'v Port>, IntrinsicError> {
    receiver_or_crash::<receiver::Port>(vm, this)
}

/// Constructs a port object from the server-described port.
pub(crate) fn create(vm: &mut Vm, port: Port) -> Value {
    object::create(vm, Kind::Port, GameObject::Port(port))
}
