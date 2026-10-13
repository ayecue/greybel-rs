//! `service` objects from `include_lib("/lib/lib<name>.so")`, one file per method.

use greybel_interpreter::{Native, Value, Vm};

use crate::game::client::receiver;
use crate::game::client::receiver::receiver;
use crate::protocol::ObjectId;

mod install;
mod start;
mod stop;

pub(crate) const METHODS: &[(&str, &Native)] = &[
    ("install_service", &install::install_service::NATIVE),
    ("start_service", &start::start_service::NATIVE),
    ("stop_service", &stop::stop_service::NATIVE),
];

/// Returns the server identity of service object `this`.
fn service(vm: &Vm, this: &Value) -> Option<ObjectId> {
    receiver::<receiver::Service>(vm, this).map(|service| service.reference())
}
