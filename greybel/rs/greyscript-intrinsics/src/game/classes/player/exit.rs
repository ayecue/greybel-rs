use greybel_interpreter::{PrintOptions, Value, Vm, native};

use crate::game::client::state::reset_custom_object;
use crate::game::game;

/// Prints nonempty `msg`, terminates the script, and drops the custom object (`PlayerUtils.ExitCommand`).
#[native]
pub fn exit(vm: &mut Vm, #[default("")] msg: &Value) {
    if !msg.is_null() {
        let message = vm.heap().display(msg).to_string();

        if !message.is_empty() {
            vm.stdout().print(&message, PrintOptions::default());
        }
    }

    // In a terminal, `exit` ends the terminal (`PlayerUtils.ExitCommand` sets `exitingBash`), which closes its
    // connection; a launched program's `exit` only stops it.
    if let Ok(game) = game(vm)
        && let Some(exit) = &game.terminal_exit
    {
        exit.set(true);
    }

    vm.exit();

    // Non-game execution has no custom object to drop.
    let _ = reset_custom_object(vm);
}
