use std::rc::Rc;

use greybel_core::BinaryOp;

use super::{CodeId, EntryCache, ImportSlot, LookupCache, Operator, UnaryOperator};
use crate::value::{Key, Number, Param, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    Outer,
    Locals,
    Globals,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Truthy {
    False,
    SemiTrue,
    True,
}

impl Truthy {
    pub fn matches_number(condition: Number, truthy: Truthy) -> bool {
        match truthy {
            Truthy::False => !condition.is_truthy(),
            Truthy::SemiTrue => condition.is_truthy(),
            Truthy::True => condition == Number::ONE,
        }
    }

    #[must_use]
    pub fn matches_bool(self, condition: bool) -> bool {
        match self {
            Truthy::False => !condition,
            Truthy::SemiTrue | Truthy::True => condition,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Operand {
    Stack,
    Value(Value),
    Slot(u32),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Destination {
    Stack,
    Slot(u32),
}

#[derive(Debug, Clone)]
pub enum VariableOperand {
    Stack,
    Value(Value),
    Variable(Box<Variable>),
}

#[derive(Debug, Clone)]
pub enum VariableDestination {
    Stack,
    Local(Box<Local>),
}

#[derive(Debug, Clone)]
pub struct Variable {
    pub key: Key,
    pub cache: LookupCache,
}

#[derive(Debug, Clone)]
pub struct Local {
    pub key: Key,
    pub cache: EntryCache,
}

#[derive(Debug, Clone)]
pub enum Op {
    Push(Value),
    Pop,
    Duplicate(u8),
    GetVariable {
        key: Key,
        invoke: bool,
        discard: bool,
        cache: LookupCache,
    },
    GetProperty {
        call: Option<u32>,
        discard: bool,
    },
    GetMember {
        key: Key,
        call: Option<u32>,
        discard: bool,
    },
    GetSelfProperty {
        key: Option<Key>,
        call: Option<u32>,
        discard: bool,
    },
    GetSuperProperty {
        key: Option<Key>,
        call: Option<u32>,
        discard: bool,
    },
    MiniScriptGetProperty {
        call: Option<u32>,
        discard: bool,
    },
    MiniScriptGetMember {
        key: Key,
        call: Option<u32>,
        discard: bool,
    },
    MiniScriptGetSelfProperty {
        key: Option<Key>,
        call: Option<u32>,
        discard: bool,
    },
    MiniScriptGetSuperProperty {
        key: Option<Key>,
        call: Option<u32>,
        discard: bool,
    },
    GetScope(Scope),
    Assign,
    AssignLocal {
        key: Key,
        cache: EntryCache,
    },
    GetSlot {
        slot: u32,
        key: Key,
        invoke: bool,
        discard: bool,
    },
    AssignSlot {
        slot: u32,
        value: Operand,
    },
    AssignMember(Key),
    Unary {
        operator: UnaryOperator,
        discard: bool,
    },
    Slice,
    Binary {
        operator: Operator,
        right: Option<Value>,
    },
    SlotBinary {
        operator: Operator,
        left: Operand,
        right: Operand,
        to: Destination,
    },
    VariableBinary {
        operator: Operator,
        left: VariableOperand,
        right: VariableOperand,
        to: VariableDestination,
    },
    AssignLocalBinary {
        key: Key,
        operator: Operator,
        right: Value,
        cache: EntryCache,
    },
    BinaryJumpIf {
        operator: Operator,
        right: Option<Value>,
        target: u32,
        when: Truthy,
    },
    SlotBinaryJumpIf {
        operator: Operator,
        left: Operand,
        right: Operand,
        target: u32,
        when: Truthy,
    },
    VariableBinaryJumpIf {
        operator: Operator,
        left: VariableOperand,
        right: VariableOperand,
        target: u32,
        when: Truthy,
    },
    CopyVariable {
        from: Box<Variable>,
        to: Box<Local>,
    },
    Comparison(Rc<[BinaryOp]>),
    MakeList {
        items: u32,
        discard: bool,
    },
    MakeMap {
        entries: u32,
        discard: bool,
    },
    Function {
        params: Rc<[Param]>,
        code: CodeId,
        capture_outer: bool,
    },
    Call {
        arguments: u32,
        discard: bool,
    },
    Return(Option<Value>),
    Jump(u32),
    JumpIf {
        target: u32,
        when: Truthy,
        push_condition: bool,
    },
    PushIterator,
    Next {
        variable: Box<Local>,
        index: Box<Local>,
        body: u32,
    },
    NextSlot {
        variable: u32,
        index: u32,
        body: u32,
    },
    PopIterator,
    Import {
        slot: ImportSlot,
        code: CodeId,
    },
    Export(ImportSlot),
    Breakpoint,
    EnableBreakpoint,
}

impl Op {
    pub(crate) fn binary(operator: Operator) -> Self {
        Self::Binary {
            operator,
            right: None,
        }
    }

    pub(crate) fn target(&self) -> Option<u32> {
        match self {
            Self::Jump(target)
            | Self::JumpIf { target, .. }
            | Self::BinaryJumpIf { target, .. }
            | Self::SlotBinaryJumpIf { target, .. }
            | Self::VariableBinaryJumpIf { target, .. }
            | Self::Next { body: target, .. }
            | Self::NextSlot { body: target, .. } => Some(*target),
            _ => None,
        }
    }

    pub(crate) fn target_mut(&mut self) -> Option<&mut u32> {
        match self {
            Self::Jump(target)
            | Self::JumpIf { target, .. }
            | Self::BinaryJumpIf { target, .. }
            | Self::SlotBinaryJumpIf { target, .. }
            | Self::VariableBinaryJumpIf { target, .. }
            | Self::Next { body: target, .. }
            | Self::NextSlot { body: target, .. } => Some(target),
            _ => None,
        }
    }
}
