//! Returns an available key without waiting, or an empty string when none is queued.

use std::rc::Rc;

use greybel_interpreter::{
    ArgContainer, Native, NativeFuture, NativeOutput, NativeParam, Value, Vm,
};

use crate::game::client::login_encryption::LoginEncryption;
use crate::game::client::stopwatch;

pub static NATIVE: Native = Native::from_async_fn(None, POLL_INPUT_PARAMS, false, call);

fn call(vm: &mut Vm, _args: &ArgContainer) -> NativeFuture {
    if let Err(error) = LoginEncryption::deny_in(vm, "poll_input") {
        return NativeOutput::ready(Err::<Value, _>(error));
    }

    if stopwatch::elapsed_ms::<PollInputWatch>(vm).is_some_and(|ms| ms < 250) {
        return NativeOutput::ready(String::new());
    }

    stopwatch::restart::<PollInputWatch>(vm);

    let stdin = Rc::clone(vm.stdin());

    Box::pin(async move {
        let result = stdin
            .try_read_key("")
            .await
            .ok()
            .flatten()
            .and_then(super::input)
            .unwrap_or_default();

        NativeOutput::new(result)
    })
}

/// Stopwatch tracking the program's previous input poll (`GreyInterpreter.pollInputWatch`).
struct PollInputWatch;

static POLL_INPUT_PARAMS: &[NativeParam] = &[];
