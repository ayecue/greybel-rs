use super::CodeBuilder;
use crate::bytecode::{
    Code, Destination, EntryCache, Local, LookupCache, Op, Operand, Source, Variable,
    VariableDestination, VariableOperand,
};
use crate::value::{Key, Value};

#[derive(Default)]
pub(super) struct FoldLog {
    starts: Vec<(u32, usize)>,
    ops: Vec<(Op, Option<Source>)>,
}

impl CodeBuilder {
    pub(super) fn fold(&mut self, op: Op, source: Option<Source>) {
        let len = self.code.ops.len();

        if !op.takes_operands()
            || !self.code.ops.last().is_some_and(Op::is_load)
            || self.is_target(len)
        {
            return self.push(op, source);
        }

        let ops = &self.code.ops;

        if len >= 2
            && !self.is_target(len - 1)
            && let Some(replacement) = op.with_operands(Some(&ops[len - 2]), &ops[len - 1])
        {
            return self.replace_tail(2, replacement, op, source);
        }

        if let Some(start_index) = op.with_operands(None, &ops[len - 1]) {
            return self.replace_tail(1, start_index, op, source);
        }

        self.push(op, source);
    }

    fn replace_tail(&mut self, count: usize, replacement: Op, op: Op, source: Option<Source>) {
        let at = self.code.ops.len() - count;

        if replacement.requires_fallback() {
            let FoldLog { starts, ops } = &mut self.fold_log;

            starts.push((Self::index(at), ops.len()));
            ops.extend(self.code.ops.drain(at..).zip(self.code.sources.drain(at..)));
            ops.push((op, source));
        } else {
            self.code.ops.truncate(at);
            self.code.sources.truncate(at);
        }

        self.push(replacement, source);
    }

    pub(super) fn is_folded(&self, index: usize) -> bool {
        self.fold_log
            .starts
            .last()
            .is_some_and(|&(start_index, _)| start_index as usize == index)
    }

    pub(super) fn extend_fold(
        &mut self,
        index: usize,
        merged_op: Op,
        op: Op,
        source: Option<Source>,
    ) {
        let originals = &mut self.fold_log.ops;
        let (last, last_source) = originals.last_mut().expect("the ops folded");

        match Self::combine(last, &op) {
            Some(new_last) => {
                *last = new_last;
                *last_source = source;
            }
            None => originals.push((op, source)),
        }

        self.code.ops[index] = merged_op;
        self.code.sources[index] = source;
    }

    pub(super) fn add_fallbacks(&mut self) {
        if self.fold_log.starts.is_empty() {
            return;
        }

        if !matches!(self.code.ops.last(), Some(Op::Return(_))) {
            self.push(Op::Jump(Code::END), None);
        }

        assert!(
            !self.is_target(self.code.ops.len()),
            "nothing jumps past the code's last op"
        );

        let FoldLog { starts, ops } = std::mem::take(&mut self.fold_log);
        let mut originals = ops.into_iter();
        let ends = starts.iter().skip(1).map(|&(_, start)| start);

        for (&(index, start), end) in starts.iter().zip(ends.chain([originals.len()])) {
            let target = self.code.ops[index as usize].target();
            let fallback = Self::index(self.code.ops.len());

            self.code.fallbacks.push((index, fallback));
            for (mut op, source) in originals.by_ref().take(end - start) {
                if let (Some(slot), Some(target)) = (op.target_mut(), target) {
                    *slot = target;
                }

                self.push(op, source);
            }

            self.push(Op::Jump(index + 1), None);
        }
    }

    fn index(len: usize) -> u32 {
        u32::try_from(len).expect("fewer than 2^32 instructions")
    }
}

enum Load {
    Value(Value),
    Slot(u32),
    Variable(Box<Variable>),
}

impl Load {
    fn slot_operand(self) -> Option<Operand> {
        match self {
            Self::Value(value) => Some(Operand::Value(value)),
            Self::Slot(slot) => Some(Operand::Slot(slot)),
            Self::Variable(_) => None,
        }
    }

    fn variable_operand(self) -> Option<VariableOperand> {
        match self {
            Self::Value(value) => Some(VariableOperand::Value(value)),
            Self::Variable(variable) => Some(VariableOperand::Variable(variable)),
            Self::Slot(_) => None,
        }
    }
}

impl Op {
    fn is_load(&self) -> bool {
        matches!(
            self,
            Self::Push(_)
                | Self::GetSlot {
                    invoke: true,
                    discard: false,
                    ..
                }
                | Self::GetVariable {
                    invoke: true,
                    discard: false,
                    ..
                }
        )
    }

    fn takes_operands(&self) -> bool {
        match self {
            Self::Binary { .. } | Self::BinaryJumpIf { .. } | Self::AssignLocal { .. } => true,
            Self::SlotBinary { left, .. } | Self::SlotBinaryJumpIf { left, .. } => {
                matches!(left, Operand::Stack)
            }
            Self::VariableBinary { left, .. } | Self::VariableBinaryJumpIf { left, .. } => {
                matches!(left, VariableOperand::Stack)
            }
            Self::AssignSlot { value, .. } => matches!(value, Operand::Stack),
            _ => false,
        }
    }

    fn load(&self) -> Option<Load> {
        match self {
            Self::Push(value) => Some(Load::Value(value.clone())),
            Self::GetSlot {
                slot,
                invoke: true,
                discard: false,
                ..
            } => Some(Load::Slot(*slot)),
            Self::GetVariable {
                key,
                invoke: true,
                discard: false,
                ..
            } => Some(Load::Variable(Box::new(Variable {
                key: key.clone(),
                cache: LookupCache::default(),
            }))),
            _ => None,
        }
    }

    fn is_slot_op(&self) -> bool {
        matches!(
            self,
            Self::SlotBinary { .. } | Self::SlotBinaryJumpIf { .. } | Self::AssignSlot { .. }
        )
    }

    fn is_variable_op(&self) -> bool {
        matches!(
            self,
            Self::VariableBinary { .. } | Self::VariableBinaryJumpIf { .. }
        )
    }

    fn with_operands(&self, below: Option<&Self>, top: &Self) -> Option<Self> {
        let top = top.load()?;
        let below = match below {
            Some(below) => Some(below.load()?),
            None => None,
        };
        let reads = |kind: fn(&Load) -> bool| kind(&top) || below.as_ref().is_some_and(kind);
        let slots = self.is_slot_op()
            || !self.is_variable_op() && reads(|load| matches!(load, Load::Slot(_)));
        let variables = self.is_variable_op()
            || !self.is_slot_op() && reads(|load| matches!(load, Load::Variable(_)));

        if slots {
            let below = match below {
                Some(below) => Some(below.slot_operand()?),
                None => None,
            };

            self.with_slot_operands(below, top.slot_operand()?)
        } else if variables {
            let below = match below {
                Some(below) => Some(below.variable_operand()?),
                None => None,
            };

            self.with_variable_operands(below, top.variable_operand()?)
        } else {
            None
        }
    }

    fn as_slot_op(&self) -> Option<Self> {
        let right = |right: &Option<Value>| right.clone().map_or(Operand::Stack, Operand::Value);

        match self {
            Self::Binary {
                operator,
                right: value,
            } => Some(Self::SlotBinary {
                operator: *operator,
                left: Operand::Stack,
                right: right(value),
                to: Destination::Stack,
            }),
            Self::BinaryJumpIf {
                operator,
                right: value,
                target,
                when,
            } => Some(Self::SlotBinaryJumpIf {
                operator: *operator,
                left: Operand::Stack,
                right: right(value),
                target: *target,
                when: *when,
            }),
            _ if self.is_slot_op() => Some(self.clone()),
            _ => None,
        }
    }

    fn with_slot_operands(&self, below: Option<Operand>, top: Operand) -> Option<Self> {
        let mut op = self.as_slot_op()?;

        match (&mut op, below) {
            (
                Self::SlotBinary { left, right, .. } | Self::SlotBinaryJumpIf { left, right, .. },
                Some(below),
            ) if matches!((&*left, &*right), (Operand::Stack, Operand::Stack)) => {
                *left = below;
                *right = top;
            }
            (
                Self::SlotBinary { left, right, .. } | Self::SlotBinaryJumpIf { left, right, .. },
                None,
            ) => match (&*left, &*right) {
                (Operand::Stack, Operand::Stack) => *right = top,
                (Operand::Stack, _) => *left = top,
                _ => return None,
            },
            (
                Self::AssignSlot {
                    value: value @ Operand::Stack,
                    ..
                },
                None,
            ) => *value = top,
            _ => return None,
        }

        Some(op)
    }

    fn as_variable_op(&self) -> Option<Self> {
        let right = |right: &Option<Value>| {
            right
                .clone()
                .map_or(VariableOperand::Stack, VariableOperand::Value)
        };

        match self {
            Self::Binary {
                operator,
                right: value,
            } => Some(Self::VariableBinary {
                operator: *operator,
                left: VariableOperand::Stack,
                right: right(value),
                to: VariableDestination::Stack,
            }),
            Self::BinaryJumpIf {
                operator,
                right: value,
                target,
                when,
            } => Some(Self::VariableBinaryJumpIf {
                operator: *operator,
                left: VariableOperand::Stack,
                right: right(value),
                target: *target,
                when: *when,
            }),
            _ if self.is_variable_op() => Some(self.clone()),
            _ => None,
        }
    }

    fn with_variable_operands(
        &self,
        below: Option<VariableOperand>,
        top: VariableOperand,
    ) -> Option<Self> {
        if let (Self::AssignLocal { key, .. }, None, VariableOperand::Variable(from)) =
            (self, &below, &top)
        {
            return Some(Self::CopyVariable {
                from: from.clone(),
                to: Local::new_box(key),
            });
        }

        let mut op = self.as_variable_op()?;

        match (&mut op, below) {
            (
                Self::VariableBinary { left, right, .. }
                | Self::VariableBinaryJumpIf { left, right, .. },
                Some(below),
            ) if matches!(
                (&*left, &*right),
                (VariableOperand::Stack, VariableOperand::Stack)
            ) =>
            {
                *left = below;
                *right = top;
            }
            (
                Self::VariableBinary { left, right, .. }
                | Self::VariableBinaryJumpIf { left, right, .. },
                None,
            ) => match (&*left, &*right) {
                (VariableOperand::Stack, VariableOperand::Stack) => *right = top,
                (VariableOperand::Stack, _) => *left = top,
                _ => return None,
            },
            _ => return None,
        }

        Some(op)
    }

    fn requires_fallback(&self) -> bool {
        match self {
            Self::SlotBinary { left, right, .. } | Self::SlotBinaryJumpIf { left, right, .. } => {
                [left, right]
                    .iter()
                    .any(|operand| matches!(operand, Operand::Slot(_)))
            }
            Self::AssignSlot { value, .. } => matches!(value, Operand::Slot(_)),
            Self::VariableBinary { left, right, .. }
            | Self::VariableBinaryJumpIf { left, right, .. } => [left, right]
                .iter()
                .any(|operand| matches!(operand, VariableOperand::Variable(_))),
            Self::CopyVariable { .. } => true,
            _ => false,
        }
    }
}

impl Local {
    pub(super) fn new_box(key: &Key) -> Box<Self> {
        Box::new(Self {
            key: key.clone(),
            cache: EntryCache::default(),
        })
    }
}
