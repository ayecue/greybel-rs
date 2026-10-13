use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ContextBreakpoint {
    pub filepath: String,
    pub line_num: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct StackItem {
    pub name: String,
    pub filepath: String,
    pub line_num: i32,
    pub is_external: bool,
}

/// Debugger step mode stored in `ContextStepRpc.mode`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StepMode {
    /// Stops at the next line regardless of function.
    Into = 0,
    /// Stops at the next line in the current function, or its caller after return.
    Over = 1,
    /// Stops at the next line after control returns to the caller.
    Out = 2,
}

/// Stack frame from `ContextBreakpointRpc.frames`, with the innermost frame first.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct DebugFrame {
    /// Display name formatted as `function <file>:<line>` for function frames or the filename for entry frames.
    pub name: String,
    pub filepath: String,
    pub line_num: i32,
    pub locals: Vec<DebugEntry>,
}

/// Debugger variable, list item, or map entry with its name.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct DebugEntry {
    pub name: String,
    /// Optional GreyScript accessor such as `[0]` or `["key"]`.
    pub accessor: Option<String>,
    pub value: DebugValue,
}

/// Value rendered by the debugger.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct DebugValue {
    /// Primitive/function type name or game-object class name.
    pub type_name: String,
    /// Truncated value rendering.
    pub text: String,
    /// Child list items or map entries; empty for non-collections.
    pub children: Vec<DebugEntry>,
    /// Indicates omitted children caused by the game's transmission-depth limit.
    pub truncated: bool,
}
