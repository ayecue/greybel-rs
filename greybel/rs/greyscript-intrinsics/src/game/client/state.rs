//! Accesses context-local VM game state and the launch tree's custom object.

use std::rc::Rc;

use greybel_interpreter::{Value, Vm};

use super::game::Game;
use super::session::Session;
use crate::error::IntrinsicError;

/// Reads `vm`'s context-local game state.
pub(crate) fn game(vm: &Vm) -> Result<&Game, IntrinsicError> {
    vm.host_data::<Game>().ok_or(IntrinsicError::NoGame)
}

pub(crate) fn session_mut(vm: &mut Vm) -> Result<&mut Session, IntrinsicError> {
    vm.host_data_mut::<Game>()
        .map(|game| &mut game.session)
        .ok_or(IntrinsicError::NoGame)
}

/// GC-roots the launch tree's current custom object, replacing the previous pin.
pub(crate) fn pin_custom_object(vm: &mut Vm) -> Result<(), IntrinsicError> {
    let game = game(vm)?;
    let current = game.custom_object.borrow().clone();
    let pins = Rc::clone(&game.custom_pin);
    let mut pin_entry = pins.borrow_mut();

    let same = match (pin_entry.as_ref(), current.as_ref()) {
        (Some((Value::Map(old), _)), Some(Value::Map(new))) => old == new,
        (None, None) => true,
        _ => false,
    };

    if same {
        return Ok(());
    }

    if let Some((_, pin)) = pin_entry.take() {
        vm.unpin(&pin);
    }

    *pin_entry = current.map(|value| (value.clone(), vm.pin(value)));

    Ok(())
}

/// Resets the shared custom object and releases its GC root (`HelperScript.ResetCustomObject`).
pub(crate) fn reset_custom_object(vm: &mut Vm) -> Result<(), IntrinsicError> {
    game(vm)?.custom_object.borrow_mut().take();

    pin_custom_object(vm)
}
