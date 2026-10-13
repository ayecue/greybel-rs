//! Checks shared by several intrinsics, which either hand values on or end the native call with its own output.

use std::ops::ControlFlow;

use greybel_interpreter::{IntoNativeResult, NativeFuture, NativeOutput};

/// Outcome of a shared check sequence: the values to continue with, or the native's final output.
pub(in crate::game) type Proceed<T> = ControlFlow<NativeFuture, T>;

/// Ends the native call with `result`.
pub(in crate::game) fn stop<T>(result: impl IntoNativeResult + 'static) -> Proceed<T> {
    ControlFlow::Break(NativeOutput::ready(result))
}

/// Unwraps a `Result<Proceed<T>, IntrinsicError>` inside a native, returning the final output when a check ended the call.
macro_rules! proceed {
    ($check:expr) => {
        match $check? {
            ::std::ops::ControlFlow::Continue(value) => value,
            ::std::ops::ControlFlow::Break(output) => return Ok(output),
        }
    };
}

pub(in crate::game) use proceed;
