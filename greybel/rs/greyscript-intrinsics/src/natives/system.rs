use std::rc::Rc;
use std::time::Duration;

use greybel_interpreter::value::CLASS_ID;
use greybel_interpreter::{Clock, FromVm, PrintOptions, RuntimeError, Sequence, Value, Vm, native};
use greybel_ms_intrinsics::natives::range::{self as base_range, NumberRange};
use greybel_ms_intrinsics::natives::system as base;

use crate::error::IntrinsicError;
use crate::game::{LoginEncryption, Terminal};

/// Minimum wait duration, matching C#'s `0.01f` comparison.
const MIN_WAIT: f64 = 0.01_f32 as f64;
const MAX_WAIT: f64 = 300.0;
/// Two-character sequence `print` converts to a line break: backslash plus `n`.
const ESCAPED_LINE_BREAK: &str = "\\n";

/// Implements `print(s, replace_text, parse_newlines)`.
/// A true `replace_text` starts a batch that replaces terminal contents (see [`TerminalOutput`](crate::game::TerminalOutput)).
/// A true `parse_newlines` converts each literal `\n` (backslash plus `n`) to a line break, except in server-side SSH encode/decode scripts, whose output bypasses the terminal (`HelperScript.AddOutput`, `GreyInterpreter.AddPendingOutput`).
#[native]
pub fn print(
    vm: &mut Vm,
    #[default("")] s: &Value,
    #[default(0)] replace_text: f64,
    #[default(1)] parse_newlines: f64,
) {
    let output = vm.heap().display(s).to_string();
    let output = if parse_newlines != 0.0 && !LoginEncryption::from_vm(vm).is_active() {
        output.replace(ESCAPED_LINE_BREAK, "\n")
    } else {
        output
    };
    let options = PrintOptions {
        new_line: true,
        replace: replace_text != 0.0,
    };

    vm.stdout().print(&output, options);
}

/// Waits 0.01 to 300 seconds after flushing program output to the terminal.
#[native]
pub async fn wait(
    #[host] clock: Rc<dyn Clock>,
    #[host] terminal: Terminal,
    #[default(1)] seconds: f64,
) -> Result<(), IntrinsicError> {
    if !(MIN_WAIT..=MAX_WAIT).contains(&seconds) {
        return Err(IntrinsicError::WaitOutOfRange);
    }

    terminal.send();

    clock.sleep(Duration::from_secs_f64(seconds)).await;

    Ok(())
}

/// MiniScript-compatible, but converts `null` to `null`.
#[native]
pub fn hash(vm: &mut Vm, obj: Option<&Value>) -> Result<Option<f64>, IntrinsicError> {
    LoginEncryption::deny_in(vm, "hash")?;

    Ok(obj.map(|obj| base::hash(vm, obj)))
}

/// MiniScript-compatible, except `from` and `to` reject `null`.
#[native]
pub fn range(
    #[default(0)] from: Option<f64>,
    #[default(0)] to: Option<f64>,
    step: &Value,
) -> Result<Sequence<NumberRange>, IntrinsicError> {
    let (Some(from), Some(to)) = (from, to) else {
        return Err(IntrinsicError::RangeNull);
    };

    let numeric_step = match step {
        Value::Number(step) => step.get(),
        _ if to >= from => 1.0,
        _ => -1.0,
    };

    // The game casts the quotient to int and then adds one. At int.MaxValue that addition wraps to a negative capacity,
    // which its allocation catch reports as `range() error`; Rust's float cast would instead report `list too large`.
    if numeric_step != 0.0 && (to - from) / numeric_step >= f64::from(i32::MAX) {
        return Err(greybel_ms_intrinsics::IntrinsicError::Range.into());
    }

    Ok(base_range::range(from, to, step)?)
}

/// Returns the value type.
/// For maps with an own or `__isa`-inherited `classID`, returns that ID, including game objects and instances created by `new` (`PlayerIntrinsics.typeof`).
#[native]
pub fn r#typeof(
    vm: &mut Vm,
    #[name = "type_object"] value: &Value,
) -> Result<String, RuntimeError> {
    let class = match value {
        Value::Map(id) => vm
            .map_lookup(*id, &Value::from(CLASS_ID))?
            .map(|(class, _)| class),
        _ => None,
    };

    Ok(match class {
        Some(class) if !class.is_null() => vm.heap().display(class).to_string(),
        _ => value.type_name().to_owned(),
    })
}
