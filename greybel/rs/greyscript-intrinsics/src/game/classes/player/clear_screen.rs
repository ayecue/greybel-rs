use greybel_interpreter::{Vm, native};

use crate::error::IntrinsicError;
use crate::game::client::login_encryption::LoginEncryption;
use crate::game::client::stopwatch;

#[native]
pub fn clear_screen(vm: &mut Vm) -> Result<(), IntrinsicError> {
    LoginEncryption::deny_in(vm, "clear_screen")?;

    if stopwatch::elapsed_ms::<ClearScreenWatch>(vm).is_some_and(|ms| ms < 100) {
        return Err(IntrinsicError::ClearScreenTooSoon);
    }

    stopwatch::restart::<ClearScreenWatch>(vm);
    vm.stdout().clear();

    Ok(())
}

/// Stopwatch tracking the program's previous screen clear (`HelperScript.GetClearScreenWatch`).
struct ClearScreenWatch;
