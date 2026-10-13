//! Shared runtime requests and debugger fixtures.

use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Waker};

use duplex_core::Error;

use super::editor::{
    Breakpoint, Continue, Evaluate, Exit, LaunchFailure, SetBreakpoints, SourceBreakpoint,
    StopReason,
};
use super::{Harness, Log, Stop};

pub const PROGRAM: &str = "f = function(x)\n  y = x * 2\n  return y\nend function\na = 1\nb = f(a)\nprint(b)\nprint(a + b)";

pub fn stop(reason: StopReason, line: u32) -> Stop {
    Stop {
        reason,
        line: Some(line),
    }
}

pub fn set_breakpoints(harness: &mut Harness, uri: &str, lines: &[u32]) {
    let breakpoints: Vec<SourceBreakpoint> = lines.iter().map(|&line| breakpoint(line)).collect();

    set_source_breakpoints(harness, uri, breakpoints);
}

/// Creates an unconditional breakpoint at `line`.
pub fn breakpoint(line: u32) -> SourceBreakpoint {
    conditional(line, None, None, None)
}

/// Sets `breakpoints` at `uri` through the runtime protocol and returns verification results.
pub fn set_source_breakpoints(
    harness: &mut Harness,
    uri: &str,
    breakpoints: Vec<SourceBreakpoint>,
) -> Vec<Breakpoint> {
    harness
        .request(&breakpoints_request(uri, breakpoints))
        .expect("set")
}

/// Builds a `SetBreakpoints` request for `uri`.
pub fn breakpoints_request(uri: &str, breakpoints: Vec<SourceBreakpoint>) -> SetBreakpoints<'_> {
    SetBreakpoints {
        uri: uri.into(),
        breakpoints: breakpoints.into(),
    }
}

pub const LOOP: &str =
    "total = 0\nfor i in range(1, 5)\n  total = total + i\nend for\nprint(total)";

pub fn conditional(
    line: u32,
    condition: Option<&str>,
    hit_condition: Option<&str>,
    log_message: Option<&str>,
) -> SourceBreakpoint {
    SourceBreakpoint {
        line,
        condition: condition.map(|text| text.to_owned()),
        hit_condition: hit_condition.map(|text| text.to_owned()),
        log_message: log_message.map(|text| text.to_owned()),
    }
}

pub fn log(text: &str, line: u32) -> Log {
    Log {
        text: text.to_owned(),
        line: Some(line),
    }
}

/// Pinned launch future polled by runtime tests.
pub type Launched = Pin<Box<dyn Future<Output = Result<Exit, Error<LaunchFailure>>>>>;

/// Continues after every stop until launch completion, collecting the evaluated `expression` value at each stop.
pub fn values_at_stops(
    harness: &mut Harness,
    launch: &mut Launched,
    expression: &str,
) -> Vec<String> {
    let mut context = Context::from_waker(Waker::noop());
    let mut values = Vec::new();

    loop {
        let stops = harness.stops.borrow().len();

        while harness.stops.borrow().len() == stops {
            if launch.as_mut().poll(&mut context).is_ready() {
                return values;
            }

            harness.run_for(1);
        }

        let value = harness
            .request(&Evaluate {
                code: expression.into(),
            })
            .expect("evaluated")
            .expect("a value");

        values.push(value.value);
        harness.editor.send(&Continue {}).expect("sent");
    }
}
