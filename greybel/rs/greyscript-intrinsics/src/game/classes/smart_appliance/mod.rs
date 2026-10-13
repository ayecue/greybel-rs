//! Smart-appliance object intrinsics (`SmartApplianceIntrinsics`).

use greybel_interpreter::{Native, Value, Vm};

use crate::game::client::receiver::{self, receiver_for};
use crate::protocol::ObjectId;
use crate::runtime::Handle;

mod model;
mod override_settings;
mod set_alarm;

pub(crate) const METHODS: &[(&str, &Native)] = &[
    ("model", &model::model::NATIVE),
    (
        "override_settings",
        &override_settings::override_settings::NATIVE,
    ),
    ("set_alarm", &set_alarm::set_alarm::NATIVE),
];

/// Returns the server identity of the appliance.
fn appliance(vm: &Vm, this: &Value, method: &str) -> Option<ObjectId> {
    receiver_for::<receiver::SmartAppliance>(vm, this, method).map(Handle::reference)
}
