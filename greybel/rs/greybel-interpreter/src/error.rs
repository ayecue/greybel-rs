use std::error::Error;
use std::fmt;
use std::rc::Rc;

use crate::bytecode::Location;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CompileError {
    #[error("Unexpected AST type {kind}")]
    UnexpectedNode {
        kind: &'static str,
        location: Location,
    },
    #[error("The module graph is empty")]
    EmptyGraph,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum RuntimeErrorKind {
    #[error("Too Many Arguments")]
    TooManyArguments,
    #[error("Max memory usage reached. Script execution interrupted")]
    MemoryLimit,
    #[error("Unable to inject into last context.")]
    NotInjectable,
    #[error("Undefined Identifier: '{0}' is unknown in this context")]
    UndefinedIdentifier(String),
    #[error("Key Not Found: '{0}' not found in map")]
    MissingKey(String),
    #[error("Type Error (while attempting to look up {0})")]
    LookupInNull(String),
    #[error("Invalid map key null")]
    NullMapKey,
    #[error("__isa depth exceeded (perhaps a reference loop?)")]
    IsaDepthLimit,
    #[error("Index Error: {kind} index ({index}) out of range ({min} to {max})")]
    IndexOutOfRange {
        kind: &'static str,
        index: i32,
        min: i64,
        max: i64,
    },
    #[error("Index Error ({kind} index {index} out of range)")]
    ElementOutOfRange { kind: &'static str, index: String },
    #[error("{kind} index must be numeric")]
    IndexNotNumeric { kind: &'static str },
    #[error("Null Reference Exception: can't index into null")]
    IndexIntoNull,
    #[error("argument to 'new' must be a map")]
    NewOfNonMap,
    #[error("{0} too large")]
    TooLarge(&'static str),
    /// A game failure, not a program failure, reported by Grey Hack using this text.
    #[error(
        "Oops, a game error has occurred that has caused a crash in the script you just ran.\n\nPlease report the error along with the steps you have taken for review.\nThank you!"
    )]
    GameCrash,
    #[error("invalid use of 'new'; to create a {}", new_of_type(*.0))]
    NewOfType(crate::vm::ValueType),
    #[error("Type Exception: can't index into this type")]
    NotIndexable,
    #[error("list item is null")]
    NullListIndex,
    #[error("Index Error (index is null)")]
    NullStringIndex,
    #[error("Index Error (list index null)")]
    NullElementIndex,
    #[error("can't set indexed element of null")]
    AssignmentToNull,
    #[error("can't set an indexed element in this type")]
    NotAssignable,
    #[error("got {operand} where a {class} was required ({operation})")]
    TypeMismatch {
        operand: &'static str,
        class: &'static str,
        operation: &'static str,
    },
    #[error("\"{parameter}\" must be a {expected_type}.")]
    InvalidArgument {
        parameter: &'static str,
        expected_type: &'static str,
    },
    #[error("{0}")]
    Native(NativeFailure),
}

pub trait NativeError: Error + 'static {
    fn is_returned(&self) -> bool {
        false
    }
}

#[derive(Clone)]
pub struct NativeFailure(Rc<dyn NativeError>);

impl NativeFailure {
    #[must_use]
    pub fn downcast_ref<E: Error + 'static>(&self) -> Option<&E> {
        (&*self.0 as &dyn Error).downcast_ref()
    }
}

impl fmt::Display for NativeFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.0, f)
    }
}

impl fmt::Debug for NativeFailure {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&self.0, f)
    }
}

impl PartialEq for NativeFailure {
    fn eq(&self, other: &Self) -> bool {
        self.0.to_string() == other.0.to_string()
    }
}

impl Eq for NativeFailure {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct IsaDepthLimit;

impl From<IsaDepthLimit> for RuntimeError {
    fn from(_: IsaDepthLimit) -> Self {
        RuntimeErrorKind::IsaDepthLimit.into()
    }
}

#[derive(Debug, thiserror::Error)]
#[error("{0}")]
struct Message(String);

impl NativeError for Message {}

impl<E: NativeError> From<E> for RuntimeError {
    fn from(error: E) -> Self {
        RuntimeErrorKind::Native(NativeFailure(Rc::new(error))).into()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{kind}")]
pub struct RuntimeError {
    pub kind: RuntimeErrorKind,
    pub native_label: Option<&'static str>,
    pub stack: Vec<Location>,
}

impl RuntimeError {
    #[must_use]
    pub fn new(message: impl Into<String>) -> Self {
        Message(message.into()).into()
    }

    #[must_use]
    pub fn returned_message(&self) -> Option<String> {
        match &self.kind {
            RuntimeErrorKind::Native(failure) if failure.0.is_returned() => {
                Some(failure.to_string())
            }
            _ => None,
        }
    }

    #[must_use]
    pub fn native<E: Error + 'static>(&self) -> Option<&E> {
        match &self.kind {
            RuntimeErrorKind::Native(failure) => failure.downcast_ref(),
            _ => None,
        }
    }

    #[must_use]
    pub fn invalid_argument(parameter: &'static str, expected_type: &'static str) -> Self {
        RuntimeErrorKind::InvalidArgument {
            parameter,
            expected_type,
        }
        .into()
    }

    #[must_use]
    pub fn with_native_label(mut self, label: &'static str) -> Self {
        self.native_label.get_or_insert(label);

        self
    }

    #[must_use]
    pub fn trace(&self) -> String {
        let native = self
            .native_label
            .map(|label| format!("at {label} (native)"));
        let frames = self.stack.iter().map(|location| format!("at {location}"));

        native
            .into_iter()
            .chain(frames)
            .collect::<Vec<_>>()
            .join("\n")
    }
}

impl From<RuntimeErrorKind> for RuntimeError {
    fn from(kind: RuntimeErrorKind) -> Self {
        Self {
            kind,
            native_label: None,
            stack: Vec::new(),
        }
    }
}

fn new_of_type(kind: crate::vm::ValueType) -> &'static str {
    match kind {
        crate::vm::ValueType::String => "string, use quotes, e.g. \"foo\"",
        crate::vm::ValueType::List => "list, use square brackets, e.g. [1,2]",
        crate::vm::ValueType::Number => "number, use a numeric literal, e.g. 42",
        crate::vm::ValueType::Function | crate::vm::ValueType::Map => {
            "function, use the 'function' keyword"
        }
    }
}
