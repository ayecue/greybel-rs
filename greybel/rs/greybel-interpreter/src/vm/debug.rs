use std::cell::{Cell, RefCell};
use std::rc::Rc;

use rustc_hash::{FxHashMap, FxHashSet};

use super::Vm;
use crate::bytecode::Location;
use crate::value::Value;

/// Debugger state shared with the host for breakpoint, step, and pause stops in debug-compiled programs. The host
/// can update it during VM execution.
#[derive(Debug, Default)]
pub struct DebugControl {
    breakpoints: RefCell<FxHashMap<Rc<str>, FxHashSet<u32>>>,
    /// Maximum call depth ([`Vm::depth`]) for the next step or pause stop. `None` disables these stops without
    /// disabling breakpoints.
    stop_depth: Cell<Option<usize>>,
    stop_reason: Cell<StopReason>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum StopReason {
    /// A step or pause stop reached its target.
    #[default]
    Step,
    /// A breakpoint stopped execution. An unfinished step remains active and can stop again until `resume` clears it.
    Breakpoint,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepMode {
    /// Stops at the next statement regardless of function.
    Into,
    /// Stops at the next statement in the current function, or its caller after return.
    Over,
    /// Stops at the next statement after control returns to the caller.
    Out,
}

impl DebugControl {
    #[must_use]
    pub fn new() -> Rc<Self> {
        Rc::new(Self::default())
    }

    pub fn set_breakpoints(&self, uri: &str, lines: impl IntoIterator<Item = u32>) {
        let lines: FxHashSet<u32> = lines.into_iter().collect();
        let mut breakpoints = self.breakpoints.borrow_mut();

        if lines.is_empty() {
            breakpoints.remove(uri);
        } else {
            breakpoints.insert(Rc::from(uri), lines);
        }
    }

    /// Requests a next-statement stop at any call depth.
    pub fn pause(&self) {
        self.stop_depth.set(Some(usize::MAX));
    }

    /// Computes the next step's stop depth from current `depth` ([`Vm::depth`]) and `mode`.
    pub fn step(&self, mode: StepMode, depth: usize) {
        self.stop_depth.set(Some(match mode {
            StepMode::Into => usize::MAX,
            StepMode::Over => depth,
            StepMode::Out => depth.saturating_sub(1),
        }));
    }

    /// Cancels pending step or pause stops and resumes until the next breakpoint.
    pub fn resume(&self) {
        self.stop_depth.set(None);
    }

    /// Returns the latest debugger stop reason.
    #[must_use]
    pub fn stop_reason(&self) -> StopReason {
        self.stop_reason.get()
    }

    fn should_stop(&self, depth: usize, location: impl FnOnce() -> Option<Location>) -> bool {
        if self
            .stop_depth
            .get()
            .is_some_and(|stop_depth| depth <= stop_depth)
        {
            self.stop_depth.set(None);
            self.stop_reason.set(StopReason::Step);

            return true;
        }

        let breakpoints = self.breakpoints.borrow();
        let at_breakpoint = !breakpoints.is_empty()
            && location().is_some_and(|location| {
                breakpoints
                    .get(&location.uri)
                    .is_some_and(|lines| lines.contains(&location.line))
            });

        if at_breakpoint {
            self.stop_reason.set(StopReason::Breakpoint);
        }

        at_breakpoint
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameView {
    /// Statement at which the frame stopped, or the call it awaits.
    pub location: Option<Location>,
    /// Location of the frame's function definition or file entry point.
    pub start: Option<Location>,
}

impl Vm {
    pub fn set_debug_control(&mut self, control: Rc<DebugControl>) {
        self.debug = Some(control);
    }

    #[must_use]
    pub fn depth(&self) -> usize {
        self.host.context_stack.stack_frame_count + self.calls.frames.len()
    }

    #[must_use]
    pub fn frames(&self) -> Vec<FrameView> {
        self.all_frames()
            .map(|(frame, _)| FrameView {
                location: self.frame_location(frame),
                start: self
                    .program
                    .code(frame.code)
                    .sources
                    .iter()
                    .flatten()
                    .next()
                    .map(|source| self.program.location(*source)),
            })
            .collect()
    }

    /// Returns frame locals in assignment order, placing `self` and `super` first when present. `index` uses
    /// [`Vm::frames`] ordering.
    #[must_use]
    pub fn frame_locals(&self, index: usize) -> Vec<(Value, Value)> {
        let Some((frame, calls)) = self.all_frames().nth(index) else {
            return Vec::new();
        };
        let mut locals: Vec<(Value, Value)> = Vec::new();
        let mut add = |name: &Value, value: &Value| {
            if !locals
                .iter()
                .any(|(local, _)| local.as_str().is_some() && local.as_str() == name.as_str())
            {
                locals.push((name.clone(), value.clone()));
            }
        };

        for (name, value) in [
            (&self.keys.self_key, &frame.self_value),
            (&self.keys.super_key, &frame.super_value),
        ] {
            if let Some(value) = value {
                add(name.value(), value);
            }
        }

        if let (Some(base), Some(slots)) = (frame.slot_base, &self.program.code(frame.code).slots) {
            for (name, value) in slots.names.iter().zip(&calls.slots[base..]) {
                if let Some(value) = value {
                    add(name.value(), value);
                }
            }
        }

        if let Some(map) = frame.locals {
            for (name, value) in map.entries(&self.heap) {
                add(name, value);
            }
        }

        locals
    }

    pub(super) fn debug_stop(&self) -> bool {
        let Some(control) = &self.debug else {
            return false;
        };

        control.should_stop(self.depth(), || self.frame_location(self.frame()))
    }

    pub(super) fn take_debug_control(&mut self) -> Option<Rc<DebugControl>> {
        self.debug.take()
    }

    pub(super) fn restore_debug_control(&mut self, control: Option<Rc<DebugControl>>) {
        self.debug = control;
    }
}
