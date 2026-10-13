//! Sandbox/test `adminNetwork` objects obtained through `admin.so` (`admin.network(public_ip)`).

use greybel_interpreter::{Native, Value, Vm};

use crate::game::client::object;
use crate::game::client::object::{GameObject, Kind};
use crate::game::client::receiver::{self, receiver_for};
use crate::protocol::PhoneAlarmConfig;

mod add_computer;
mod computer;
mod fire_admin;
mod fire_npc;
mod hire_admin;
mod hire_crown_shield;
mod hire_npc;
mod phone_alarm;
mod remove;
mod report_library_update;
mod set_admin_restore;
mod set_company_shares;
mod set_net_speed;
mod set_phone_alarm;

pub(crate) const METHODS: &[(&str, &Native)] = &[
    ("computer", &computer::computer::NATIVE),
    ("add_computer", &add_computer::add_computer::NATIVE),
    ("fire_npc", &fire_npc::fire_npc::NATIVE),
    ("hire_npc", &hire_npc::hire_npc::NATIVE),
    ("hire_admin", &hire_admin::hire_admin::NATIVE),
    ("fire_admin", &fire_admin::fire_admin::NATIVE),
    ("remove", &remove::remove::NATIVE),
    ("set_net_speed", &set_net_speed::set_net_speed::NATIVE),
    ("set_phone_alarm", &set_phone_alarm::set_phone_alarm::NATIVE),
    ("phone_alarm", &phone_alarm::phone_alarm::NATIVE),
    (
        "report_library_update",
        &report_library_update::report_library_update::NATIVE,
    ),
    (
        "set_company_shares",
        &set_company_shares::set_company_shares::NATIVE,
    ),
    (
        "hire_crown_shield",
        &hire_crown_shield::hire_crown_shield::NATIVE,
    ),
    (
        "set_admin_restore",
        &set_admin_restore::set_admin_restore::NATIVE,
    ),
];

pub(crate) fn create(vm: &mut Vm, public_ip: String) -> Value {
    object::create(vm, Kind::AdminNetwork, GameObject::AdminNetwork(public_ip))
}

/// Returns this network's public address when `this` is an `adminNetwork` providing `method`.
fn public_ip(vm: &Vm, this: &Value, method: &str) -> Option<String> {
    receiver_for::<receiver::AdminNetwork>(vm, this, method).cloned()
}

/// Converts a smartphone alarm into the map `phone_alarm` and `set_phone_alarm` return, or null without one.
fn alarm_map(vm: &mut Vm, alarm: Option<PhoneAlarmConfig>) -> Value {
    let heap = vm.heap_mut();

    alarm.map_or(Value::Null, |alarm| {
        Value::Map(heap.add_map_from([
            (Value::from("hour"), Value::from(f64::from(alarm.hour))),
            (Value::from("minute"), Value::from(f64::from(alarm.minute))),
            (Value::from("isAlarmModified"), Value::from(alarm.modified)),
            (Value::from("isAlarmDeleted"), Value::from(alarm.deleted)),
            (
                Value::from("lateStrikes"),
                Value::from(f64::from(alarm.late_strikes)),
            ),
            (Value::from("fired"), Value::from(alarm.fired)),
        ]))
    })
}
