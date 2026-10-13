use greybel_interpreter::value::CLASS_ID;
use greybel_interpreter::{Value, Vm, native};
use greybel_symbols::names::ISA;

use crate::error::IntrinsicError;
use crate::game::client::login_encryption::LoginEncryption;
use crate::game::client::state::pin_custom_object;
use crate::game::game;

/// Returns the launch tree's custom object, creating it once and sharing it with launched programs (`get_custom_object`).
#[native]
pub fn get_custom_object(vm: &mut Vm) -> Result<Value, IntrinsicError> {
    LoginEncryption::deny_in(vm, "get_custom_object")?;

    let game = game(vm)?.clone();

    if let Some(value) = game.custom_object.borrow().clone() {
        return Ok(value);
    }

    let isa = vm.heap_mut().add_map_from(Vec::new());
    let custom = vm.heap_mut().add_map_from([
        (Value::from(ISA), Value::Map(isa)),
        (Value::from(CLASS_ID), Value::from("custom_object")),
    ]);
    let custom = Value::Map(custom);

    *game.custom_object.borrow_mut() = Some(custom.clone());
    pin_custom_object(vm)?;

    Ok(custom)
}
