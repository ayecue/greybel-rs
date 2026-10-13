use greyhack_message_hook_client::{DebugEntry, DebugFrame, DebugValue};

use crate::protocol::{
    Evaluation, EvaluationFailure, InspectionFailure, Scope, SourceLocation, StackFrame, Variable,
};

/// Game stop state and editor references retained until resume.
pub(super) struct GameInspection {
    frames: Vec<DebugFrame>,
    globals: Vec<DebugEntry>,
    references: Vec<Reference>,
}

enum Reference {
    Locals(usize),
    Globals,
    Value {
        value: DebugValue,
        /// Optional expression for reading the value in the innermost frame.
        evaluate_name: Option<String>,
    },
}

impl GameInspection {
    pub(super) fn new(frames: Vec<DebugFrame>, globals: Vec<DebugEntry>) -> Self {
        Self {
            frames,
            globals,
            references: Vec::new(),
        }
    }

    pub(super) fn stack_trace(&self) -> Vec<StackFrame> {
        self.frames
            .iter()
            .enumerate()
            .map(|(index, frame)| StackFrame {
                id: u32::try_from(index).unwrap_or(u32::MAX),
                name: frame.name.clone(),
                location: Some(SourceLocation::in_game(&frame.filepath, frame.line_num)),
            })
            .collect()
    }

    pub(super) fn scopes(&mut self, frame: u32) -> Result<Vec<Scope>, InspectionFailure> {
        let frame = frame as usize;

        if frame >= self.frames.len() {
            return Err(InspectionFailure::Unknown);
        }

        Ok(vec![
            Scope {
                name: "Locals".into(),
                reference: self.add(Reference::Locals(frame)),
            },
            Scope {
                name: "Globals".into(),
                reference: self.add(Reference::Globals),
            },
        ])
    }

    /// Returns the reevaluation expression used to fetch children of a game-truncated referenced value.
    pub(super) fn children_expression(&self, reference: u32) -> Option<String> {
        match self.reference(reference).ok()? {
            Reference::Value {
                value,
                evaluate_name: Some(evaluate_name),
            } if value.truncated => Some(evaluate_name.clone()),
            _ => None,
        }
    }

    /// Replaces a referenced `DebugValue` with its full reevaluated value.
    pub(super) fn take_children(&mut self, reference: u32, value: DebugValue) {
        let index = (reference as usize).wrapping_sub(1);

        if let Some(Reference::Value { value: current, .. }) = self.references.get_mut(index) {
            *current = value;
        }
    }

    pub(super) fn variables(&mut self, reference: u32) -> Result<Vec<Variable>, InspectionFailure> {
        let entries: Vec<(DebugEntry, Option<String>)> = match self.reference(reference)? {
            Reference::Locals(frame) => self.frames[*frame]
                .locals
                .iter()
                .map(|entry| (entry.clone(), (*frame == 0).then(|| entry.name.clone())))
                .collect(),
            Reference::Globals => self
                .globals
                .iter()
                .map(|entry| {
                    (
                        entry.clone(),
                        entry
                            .accessor
                            .as_ref()
                            .map(|accessor| format!("globals{accessor}")),
                    )
                })
                .collect(),
            Reference::Value {
                value,
                evaluate_name,
            } => value
                .children
                .iter()
                .map(|entry| {
                    let name = evaluate_name
                        .as_ref()
                        .zip(entry.accessor.as_ref())
                        .map(|(parent, accessor)| format!("{parent}{accessor}"));

                    (entry.clone(), name)
                })
                .collect(),
        };

        Ok(entries
            .into_iter()
            .map(|(entry, evaluate_name)| self.variable(entry.name, entry.value, evaluate_name))
            .collect())
    }

    /// Creates a protocol evaluation and reference for an expression result.
    pub(super) fn evaluation(&mut self, value: DebugValue, code: &str) -> Evaluation {
        let reference = self.value_reference(&value, Some(format!("({code})")));

        Evaluation {
            value: value.text,
            type_name: value.type_name,
            reference,
        }
    }

    /// Creates a protocol variable and reference from its current value and optional read expression.
    pub(super) fn variable(
        &mut self,
        name: String,
        value: DebugValue,
        evaluate_name: Option<String>,
    ) -> Variable {
        let reference = self.value_reference(&value, evaluate_name.clone());

        Variable {
            name,
            value: value.text,
            type_name: value.type_name,
            reference,
            evaluate_name,
        }
    }

    /// Builds assignments for innermost locals, globals, or referenced collection entries; rejects other scopes.
    pub(super) fn assignable(
        &self,
        reference: u32,
        name: &str,
    ) -> Result<String, EvaluationFailure> {
        let unassignable = || EvaluationFailure::CodeError {
            message: format!(
                "{name} can't be set here: only the innermost frame's and global variables can."
            ),
        };
        let accessor = |entries: &[DebugEntry]| {
            entries
                .iter()
                .find(|entry| entry.name == name)
                .and_then(|entry| entry.accessor.clone())
        };

        match self.reference(reference).map_err(|_| unassignable())? {
            Reference::Locals(0) => Ok(name.to_owned()),
            Reference::Globals => accessor(&self.globals)
                .map(|accessor| format!("globals{accessor}"))
                .ok_or_else(unassignable),
            Reference::Value {
                value,
                evaluate_name: Some(parent),
            } => accessor(&value.children)
                .map(|accessor| format!("{parent}{accessor}"))
                .ok_or_else(unassignable),
            Reference::Locals(_) | Reference::Value { .. } => Err(unassignable()),
        }
    }

    fn value_reference(&mut self, value: &DebugValue, evaluate_name: Option<String>) -> u32 {
        if value.children.is_empty() && !(value.truncated && evaluate_name.is_some()) {
            return 0;
        }

        self.add(Reference::Value {
            value: value.clone(),
            evaluate_name,
        })
    }

    fn reference(&self, reference: u32) -> Result<&Reference, InspectionFailure> {
        let index = (reference as usize)
            .checked_sub(1)
            .ok_or(InspectionFailure::Unknown)?;

        self.references.get(index).ok_or(InspectionFailure::Unknown)
    }

    fn add(&mut self, reference: Reference) -> u32 {
        self.references.push(reference);

        u32::try_from(self.references.len()).unwrap_or(u32::MAX)
    }
}
