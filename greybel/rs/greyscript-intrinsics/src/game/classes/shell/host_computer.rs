use greybel_interpreter::{Value, Vm, native};

use super::shell;
use crate::game::client::object;

/// Returns the shell's host computer accessed as the shell user.
#[native(method)]
pub fn host_computer(vm: &mut Vm, this: &Value) -> Value {
    shell(vm, this, "host_computer").map_or(Value::Null, |access| {
        object::create_computer(vm, access.host_computer())
    })
}
