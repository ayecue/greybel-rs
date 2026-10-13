use greybel_interpreter::{Pin, Value, Vm};

use super::entry::Entry;
use crate::protocol::{
    Evaluation, EvaluationFailure, InspectionFailure, Scope, SourceLocation, StackFrame, Variable,
};

/// Debugger references for stopped scopes and values. Heap values stay pinned until resume to prevent evaluated
/// code from collecting them.
#[derive(Default)]
pub(super) struct MockInspection {
    references: Vec<Reference>,
}

enum Reference {
    Locals(usize),
    Globals,
    Value {
        pin: Pin,
        /// Optional expression for reading the value in the innermost frame.
        evaluate_name: Option<String>,
    },
}

impl MockInspection {
    pub(super) fn stack_trace(vm: &Vm) -> Vec<StackFrame> {
        let frames = vm.frames();
        let entry = frames.len().saturating_sub(1);

        frames
            .into_iter()
            .enumerate()
            .map(|(index, frame)| {
                let file = frame
                    .location
                    .as_ref()
                    .or(frame.start.as_ref())
                    .map_or("", |location| file_name(&location.uri));
                let name = match &frame.start {
                    Some(start) if index != entry => {
                        format!("function {}:{}", file_name(&start.uri), start.line)
                    }
                    _ => file.to_owned(),
                };

                StackFrame {
                    id: u32::try_from(index).unwrap_or(u32::MAX),
                    name,
                    location: frame.location.as_ref().map(SourceLocation::from),
                }
            })
            .collect()
    }

    pub(super) fn scopes(&mut self, vm: &Vm, frame: u32) -> Result<Vec<Scope>, InspectionFailure> {
        let frame = frame as usize;

        if frame >= vm.depth() {
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

    pub(super) fn variables(
        &mut self,
        vm: &mut Vm,
        reference: u32,
    ) -> Result<Vec<Variable>, InspectionFailure> {
        let entries = self.entries(vm, reference)?;

        Ok(entries
            .into_iter()
            .map(|entry| self.variable(vm, entry))
            .collect())
    }

    /// Converts a pinned expression result to a protocol evaluation. Lists/maps retain references; scalars are unpinned.
    pub(super) fn evaluation(&mut self, vm: &mut Vm, pin: Pin, code: &str) -> Evaluation {
        let value = vm.pinned_value(&pin).clone();
        let reference = match value {
            Value::List(_) | Value::Map(_) => self.add(Reference::Value {
                pin,
                evaluate_name: Some(format!("({code})")),
            }),
            _ => {
                vm.unpin(pin);

                0
            }
        };

        Evaluation {
            value: Entry::display(vm.heap(), &value),
            type_name: value.type_name().into(),
            reference,
        }
    }

    /// Finds variable `name` in the referenced scope or value.
    pub(super) fn find_variable(
        &mut self,
        vm: &mut Vm,
        reference: u32,
        name: &str,
    ) -> Result<Variable, EvaluationFailure> {
        let entries = self
            .entries(vm, reference)
            .map_err(|_| unknown_variable(name))?;
        let entry = entries
            .into_iter()
            .find(|entry| entry.name == name)
            .ok_or_else(|| unknown_variable(name))?;

        Ok(self.variable(vm, entry))
    }

    /// Builds assignments for innermost locals, globals, or referenced collection entries; rejects other scopes.
    pub(super) fn assignable(
        &self,
        vm: &Vm,
        reference: u32,
        name: &str,
    ) -> Result<String, EvaluationFailure> {
        let unassignable = || EvaluationFailure::CodeError {
            message: format!(
                "{name} can't be set here: only the innermost frame's and global variables can."
            ),
        };
        let index = (reference as usize)
            .checked_sub(1)
            .ok_or_else(unassignable)?;

        match self.references.get(index).ok_or_else(unassignable)? {
            Reference::Locals(0) => Ok(name.to_owned()),
            Reference::Globals => Ok(format!("globals{}", Entry::string_accessor(name))),
            Reference::Value {
                pin,
                evaluate_name: Some(parent),
            } => match vm.pinned_value(pin) {
                Value::List(_) => Ok(format!("{parent}[{name}]")),
                _ => Ok(format!("{parent}{}", Entry::string_accessor(name))),
            },
            Reference::Locals(_) | Reference::Value { .. } => Err(unassignable()),
        }
    }

    /// Releases all pinned heap values at resume.
    pub(super) fn release(self, vm: &mut Vm) {
        for reference in self.references {
            if let Reference::Value { pin, .. } = reference {
                vm.unpin(pin);
            }
        }
    }

    fn entries(&self, vm: &Vm, reference: u32) -> Result<Vec<Entry>, InspectionFailure> {
        let index = (reference as usize)
            .checked_sub(1)
            .ok_or(InspectionFailure::Unknown)?;
        let heap = vm.heap();

        Ok(
            match self
                .references
                .get(index)
                .ok_or(InspectionFailure::Unknown)?
            {
                Reference::Locals(frame) => vm
                    .frame_locals(*frame)
                    .into_iter()
                    .map(|(name, value)| {
                        let name = heap.display(&name).to_string();

                        Entry {
                            evaluate_name: (*frame == 0).then(|| name.clone()),
                            name,
                            value,
                        }
                    })
                    .collect(),
                Reference::Globals => {
                    Entry::of_map(heap, vm.globals().entries(heap).iter(), Some("globals"))
                }
                Reference::Value { pin, evaluate_name } => match vm.pinned_value(pin) {
                    Value::List(list) => list
                        .items(heap)
                        .iter()
                        .enumerate()
                        .map(|(index, value)| Entry {
                            name: index.to_string(),
                            evaluate_name: evaluate_name
                                .as_ref()
                                .map(|parent| format!("{parent}[{index}]")),
                            value: value.clone(),
                        })
                        .collect(),
                    Value::Map(map) => {
                        Entry::of_map(heap, map.entries(heap).iter(), evaluate_name.as_deref())
                    }
                    _ => Vec::new(),
                },
            },
        )
    }

    fn variable(&mut self, vm: &mut Vm, entry: Entry) -> Variable {
        let Entry {
            name,
            evaluate_name,
            value,
        } = entry;
        let text = Entry::display(vm.heap(), &value);
        let type_name = value.type_name().into();
        let reference = match value {
            Value::List(_) | Value::Map(_) => {
                let pin = vm.pin(value);

                self.add(Reference::Value {
                    pin,
                    evaluate_name: evaluate_name.clone(),
                })
            }
            _ => 0,
        };

        Variable {
            name,
            value: text,
            type_name,
            reference,
            evaluate_name,
        }
    }

    fn add(&mut self, reference: Reference) -> u32 {
        self.references.push(reference);

        u32::try_from(self.references.len()).unwrap_or(u32::MAX)
    }
}

fn unknown_variable(name: &str) -> EvaluationFailure {
    EvaluationFailure::CodeError {
        message: format!("{name} isn't there anymore."),
    }
}

fn file_name(uri: &str) -> &str {
    uri.rsplit('/').next().unwrap_or(uri)
}
