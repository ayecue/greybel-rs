mod expression;
mod operands;
mod slots;
mod statement;

use std::collections::HashMap;
use std::path::Path;
use std::rc::Rc;

use greybel_core::{Ast, ColumnSettings, Dialect, NodeId, NodeKind};
use greybel_documents::{ModuleGraph, ModuleId, NativeImportMode};
use greybel_symbols::names::{EXPORTS, LOOP_INDEX_PREFIX, LOOP_INDEX_SUFFIX, MODULE};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::bytecode::{
    Code, CodeId, Destination, EntryCache, File, FileId, ImportSlot, Local, Location, LookupCache,
    Op, Operand, Program, Source, Truthy, UnaryOperator, VariableDestination,
};
use crate::error::CompileError;
use crate::value::{Key, Text, Value};

pub(crate) type CompileResult<T = ()> = Result<T, Box<CompileError>>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Usage {
    Statement,
    Value,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CompileOptions {
    pub environment: HashMap<String, String>,
    pub debug: bool,
    pub dialect: Dialect,
    pub native_imports: NativeImportMode,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct Label(u32);

struct CodeBuilder {
    code: Code,
    slots: Option<slots::SlotBuilder>,
    labels: Vec<Option<u32>>,
    label_targets: Vec<u32>,
    jumps: Vec<(usize, Label)>,
    loops: Vec<(Label, Label)>,
    fold_log: operands::FoldLog,
    top_level: bool,
}

impl CodeBuilder {
    fn new(top_level: bool) -> Self {
        Self {
            code: Code::default(),
            slots: None,
            labels: Vec::new(),
            label_targets: Vec::new(),
            jumps: Vec::new(),
            loops: Vec::new(),
            fold_log: operands::FoldLog::default(),
            top_level,
        }
    }

    fn is_target(&self, index: usize) -> bool {
        self.label_targets
            .iter()
            .rev()
            .map(|&target| target as usize)
            .take_while(|&target| target >= index)
            .any(|target| target == index)
    }

    fn insert(&mut self, op: Op, source: Option<Source>) {
        let len = self.code.ops.len();

        if len == 0 || self.is_target(len) {
            return self.push(op, source);
        }

        let Some(merged_op) = Self::combine(&self.code.ops[len - 1], &op) else {
            return self.fold(op, source);
        };

        if self.is_folded(len - 1) {
            return self.extend_fold(len - 1, merged_op, op, source);
        }

        self.code.ops.pop();
        self.code.sources.pop();
        self.fold(merged_op, source);
    }

    fn push(&mut self, op: Op, source: Option<Source>) {
        self.code.ops.push(op);
        self.code.sources.push(source);
    }

    fn combine(last: &Op, op: &Op) -> Option<Op> {
        match (last, op) {
            (Op::Push(_), Op::Binary { .. }) | (Op::Binary { .. }, _) => {
                Self::combine_binary(last, op)
            }
            (Op::SlotBinary { .. }, _) => Self::combine_slot_binary(last, op),
            (Op::VariableBinary { .. }, _) => Self::combine_variable_binary(last, op),
            (Op::Push(value), Op::Return(None)) => Some(Op::Return(Some(value.clone()))),
            _ => None,
        }
    }

    fn combine_binary(last: &Op, op: &Op) -> Option<Op> {
        match (last, op) {
            (
                Op::Push(right),
                Op::Binary {
                    operator,
                    right: None,
                },
            ) => Some(Op::Binary {
                operator: *operator,
                right: Some(right.clone()),
            }),
            (
                Op::Binary { operator, right },
                Op::JumpIf {
                    target,
                    when,
                    push_condition: false,
                },
            ) => Some(Op::BinaryJumpIf {
                operator: *operator,
                right: right.clone(),
                target: *target,
                when: *when,
            }),
            (
                Op::Binary {
                    operator,
                    right: Some(right),
                },
                Op::AssignLocal { key, .. },
            ) => Some(Op::AssignLocalBinary {
                key: key.clone(),
                operator: *operator,
                right: right.clone(),
                cache: EntryCache::default(),
            }),
            (
                Op::Binary { operator, right },
                Op::AssignSlot {
                    slot,
                    value: Operand::Stack,
                },
            ) => Some(Op::SlotBinary {
                operator: *operator,
                left: Operand::Stack,
                right: right.clone().map_or(Operand::Stack, Operand::Value),
                to: Destination::Slot(*slot),
            }),
            _ => None,
        }
    }

    fn combine_slot_binary(last: &Op, op: &Op) -> Option<Op> {
        match (last, op) {
            (
                Op::SlotBinary {
                    operator,
                    left,
                    right,
                    to: Destination::Stack,
                },
                Op::JumpIf {
                    target,
                    when,
                    push_condition: false,
                },
            ) => Some(Op::SlotBinaryJumpIf {
                operator: *operator,
                left: left.clone(),
                right: right.clone(),
                target: *target,
                when: *when,
            }),
            (
                Op::SlotBinary {
                    operator,
                    left,
                    right,
                    to: Destination::Stack,
                },
                Op::AssignSlot {
                    slot,
                    value: Operand::Stack,
                },
            ) => Some(Op::SlotBinary {
                operator: *operator,
                left: left.clone(),
                right: right.clone(),
                to: Destination::Slot(*slot),
            }),
            _ => None,
        }
    }

    fn combine_variable_binary(last: &Op, op: &Op) -> Option<Op> {
        match (last, op) {
            (
                Op::VariableBinary {
                    operator,
                    left,
                    right,
                    to: VariableDestination::Stack,
                },
                Op::JumpIf {
                    target,
                    when,
                    push_condition: false,
                },
            ) => Some(Op::VariableBinaryJumpIf {
                operator: *operator,
                left: left.clone(),
                right: right.clone(),
                target: *target,
                when: *when,
            }),
            (
                Op::VariableBinary {
                    operator,
                    left,
                    right,
                    to: VariableDestination::Stack,
                },
                Op::AssignLocal { key, .. },
            ) => Some(Op::VariableBinary {
                operator: *operator,
                left: left.clone(),
                right: right.clone(),
                to: VariableDestination::Local(Local::new_box(key)),
            }),
            _ => None,
        }
    }

    fn build(mut self) -> Code {
        for (index, label) in &self.jumps {
            let target = self.labels[label.0 as usize].expect("every label is placed");

            if let Some(slot) = self.code.ops[*index].target_mut() {
                *slot = target;
            }
        }

        self.add_fallbacks();

        self.code
    }
}

pub struct Compiler<'g> {
    graph: &'g ModuleGraph,
    options: &'g CompileOptions,
    codes: Vec<Option<Code>>,
    files: Vec<File>,
    file_ids: FxHashMap<ModuleId, FileId>,
    imports: FxHashMap<ModuleId, (ImportSlot, CodeId)>,
    warnings: Vec<String>,
    modules: Vec<ModuleId>,
    code_builder: Vec<CodeBuilder>,
    included_native_imports: FxHashSet<ModuleId>,
    strings: FxHashSet<Rc<str>>,
    texts: FxHashMap<Rc<str>, Text>,
    /// Optional chains being compiled, innermost last.
    chains: Vec<OptionalChain>,
}

/// A chain with `?.` links (`a?.b.c`): a link whose base is `null` jumps to `end` with that `null` as the chain's
/// value.
struct OptionalChain {
    end: Label,
    links: Vec<NodeId>,
}

impl Program {
    pub fn compile(graph: &ModuleGraph, options: &CompileOptions) -> Result<Self, CompileError> {
        Compiler::compile(graph, options).map_err(|error| *error)
    }
}

impl<'g> Compiler<'g> {
    fn compile(graph: &'g ModuleGraph, options: &'g CompileOptions) -> CompileResult<Program> {
        let entry = graph.entry().ok_or(CompileError::EmptyGraph)?;
        let mut compiler = Self {
            graph,
            options,
            codes: Vec::new(),
            files: Vec::new(),
            file_ids: FxHashMap::default(),
            imports: FxHashMap::default(),
            warnings: Vec::new(),
            modules: Vec::new(),
            code_builder: Vec::new(),
            included_native_imports: FxHashSet::default(),
            strings: FxHashSet::default(),
            texts: FxHashMap::default(),
            chains: Vec::new(),
        };
        let code = compiler.allocate_code();

        compiler.modules.push(entry);
        compiler.code_builder.push(CodeBuilder::new(true));
        compiler.body(graph.module(entry).ast.body())?;
        compiler.finish_code_builder(code);

        let imports = u32::try_from(compiler.imports.len()).expect("fewer than 2^32 imports");

        Ok(Program {
            codes: compiler
                .codes
                .into_iter()
                .map(|code| code.expect("every reserved code is filled"))
                .collect(),
            entry: code,
            files: compiler.files,
            import_count: imports,
            warnings: compiler.warnings,
            dialect: options.dialect,
        })
    }

    pub(crate) fn loop_index(variable: &str) -> String {
        format!("{LOOP_INDEX_PREFIX}{variable}{LOOP_INDEX_SUFFIX}")
    }

    pub(crate) fn ast(&self) -> &'g Ast {
        &self.graph.module(self.module()).ast
    }

    #[must_use]
    pub fn graph(&self) -> &'g ModuleGraph {
        self.graph
    }

    #[must_use]
    pub fn module(&self) -> ModuleId {
        *self.modules.last().expect("a file is being compiled")
    }

    pub(crate) fn shared_str(&mut self, text: &str) -> Rc<str> {
        if let Some(string_handle) = self.strings.get(text) {
            return Rc::clone(string_handle);
        }

        let string_handle: Rc<str> = text.into();

        self.strings.insert(Rc::clone(&string_handle));

        string_handle
    }

    pub(crate) fn text(&mut self, text: &str) -> Text {
        if let Some(string_handle) = self.texts.get(text) {
            return string_handle.clone();
        }

        let name = self.shared_str(text);

        self.texts
            .entry(name)
            .or_insert_with(|| Text::new(text))
            .clone()
    }

    pub(crate) fn key(&mut self, text: &str) -> Key {
        Key::new(self.text(text))
    }

    pub(crate) fn kind(&self, id: NodeId) -> &'g NodeKind {
        self.ast().kind(id)
    }

    pub(crate) fn unwrap(&self, mut id: NodeId) -> NodeId {
        while let NodeKind::Parenthesis(node) = self.kind(id) {
            id = node.expression;
        }

        id
    }

    pub(crate) fn name(&self, id: NodeId) -> Option<&'g str> {
        match self.kind(id) {
            NodeKind::Identifier(node) => Some(self.ast().resolve(node.name)),
            _ => None,
        }
    }

    fn file(&mut self, module: ModuleId) -> FileId {
        if let Some(&id) = self.file_ids.get(&module) {
            return id;
        }

        let id = FileId(u32::try_from(self.files.len()).expect("fewer than 2^32 files"));
        let module_ref = self.graph.module(module);

        self.files.push(File {
            path: module_ref.uri.label().into(),
            uri: module_ref.uri.to_string().into(),
            lines: module_ref.ast.lines().clone(),
        });
        self.file_ids.insert(module, id);

        id
    }

    pub(crate) fn location(&self, id: NodeId) -> Location {
        let module = self.graph.module(self.module());
        let position = module
            .ast
            .lines()
            .position(module.ast.span(id).start, ColumnSettings::default());

        Location {
            path: module.uri.label().into(),
            uri: module.uri.to_string().into(),
            line: position.line,
            character: position.character,
        }
    }

    fn code_builder(&mut self) -> &mut CodeBuilder {
        self.code_builder
            .last_mut()
            .expect("a code block is being built")
    }

    pub fn emit(&mut self, op: Op, node: NodeId) {
        let source = Source {
            file: self.file(self.module()),
            span: self.ast().span(node),
        };
        let op = self.map_to_dialect(op);

        self.code_builder().insert(op, Some(source));
    }

    pub fn emit_internal(&mut self, op: Op) {
        let op = self.map_to_dialect(op);
        let code = &mut self.code_builder().code;

        code.ops.push(op);
        code.sources.push(None);
    }

    fn map_to_dialect(&self, op: Op) -> Op {
        if self.options.dialect != Dialect::MiniScript {
            return op;
        }

        match op {
            Op::GetProperty { call, discard } => Op::MiniScriptGetProperty { call, discard },
            Op::GetMember { key, call, discard } => Op::MiniScriptGetMember { key, call, discard },
            Op::GetSelfProperty { key, call, discard } => {
                Op::MiniScriptGetSelfProperty { key, call, discard }
            }
            Op::GetSuperProperty { key, call, discard } => {
                Op::MiniScriptGetSuperProperty { key, call, discard }
            }
            Op::Unary {
                operator: UnaryOperator::New,
                discard,
            } => Op::Unary {
                operator: UnaryOperator::MiniScriptNew,
                discard,
            },
            op => op,
        }
    }

    pub(crate) fn push(&mut self, value: impl Into<Value>, node: NodeId) {
        let value = match value.into() {
            Value::String(text) => Value::String(self.text(&text)),
            other => other,
        };

        self.emit(Op::Push(value), node);
    }

    pub(crate) fn label(&mut self) -> Label {
        let code_builder = self.code_builder();

        code_builder.labels.push(None);

        Label(u32::try_from(code_builder.labels.len() - 1).expect("fewer than 2^32 labels"))
    }

    pub(crate) fn set_label(&mut self, label: Label) {
        let code_builder = self.code_builder();
        let here =
            u32::try_from(code_builder.code.ops.len()).expect("fewer than 2^32 instructions");

        code_builder.labels[label.0 as usize] = Some(here);
        code_builder.label_targets.push(here);
    }

    pub(crate) fn jump(&mut self, op: Op, label: Label, node: NodeId) {
        self.emit(op, node);

        let block = self.code_builder();
        let index = block.code.ops.len() - 1;

        block.jumps.push((index, label));
    }

    pub(crate) fn jump_if_false() -> Op {
        Op::JumpIf {
            target: 0,
            when: Truthy::False,
            push_condition: false,
        }
    }

    pub(crate) fn loops(&mut self) -> &mut Vec<(Label, Label)> {
        &mut self.code_builder().loops
    }

    fn allocate_code(&mut self) -> CodeId {
        self.codes.push(None);

        CodeId(u32::try_from(self.codes.len() - 1).expect("fewer than 2^32 code blocks"))
    }

    fn finish_code_builder(&mut self, id: CodeId) {
        let block = self.code_builder.pop().expect("a block is being built");

        self.codes[id.0 as usize] = Some(block.build());
    }

    pub(crate) fn add_code_builder(
        &mut self,
        top_level: bool,
        build: impl FnOnce(&mut Self) -> CompileResult,
    ) -> CompileResult<CodeId> {
        let id = self.open_code_builder(top_level);

        build(self)?;
        self.finish_code_builder(id);

        Ok(id)
    }

    fn open_code_builder(&mut self, top_level: bool) -> CodeId {
        let id = self.allocate_code();

        self.code_builder.push(CodeBuilder::new(top_level));

        id
    }

    pub(crate) fn use_slots(&mut self, layout: slots::SlotBuilder, params: &[crate::value::Param]) {
        let block = self.code_builder();

        block.code.slots = Some(layout.finish(params));
        block.slots = Some(layout);
    }

    pub(crate) fn slot(&self, name: &str) -> Option<u32> {
        self.code_builder.last()?.slots.as_ref()?.slot(name)
    }

    pub(crate) fn is_top_level(&self) -> bool {
        self.code_builder
            .last()
            .is_some_and(|block| block.top_level)
    }

    pub(crate) fn body(&mut self, body: &[NodeId]) -> CompileResult {
        for &id in body {
            self.statement(id)?;
        }

        Ok(())
    }

    pub fn dependency(&mut self, node: NodeId) -> Option<ModuleId> {
        let target = self
            .graph
            .module(self.module())
            .dependency(node)
            .expect("the loader resolved every directive");

        if self.modules.contains(&target) {
            let line = self.location(node).line;

            self.warnings.push(format!(
                "Found circular dependency between \"{}\" and \"{}\" at line {line}. Using noop instead to prevent overflow.",
                self.graph.module(self.module()).uri.label(),
                self.graph.module(target).uri.label(),
            ));

            return None;
        }

        Some(target)
    }

    pub fn include(&mut self, module: ModuleId) -> CompileResult {
        self.modules.push(module);

        let result = self.body(self.graph.module(module).ast.body());

        self.modules.pop();

        result
    }

    pub(crate) fn import(&mut self, module: ModuleId) -> CompileResult<(ImportSlot, CodeId)> {
        if let Some(&import) = self.imports.get(&module) {
            return Ok(import);
        }

        self.modules.push(module);

        let code = self.add_code_builder(true, |c| {
            c.emit_internal(Op::Push(EXPORTS.into()));
            c.emit_internal(Op::MakeMap {
                entries: 0,
                discard: false,
            });
            c.emit_internal(Op::MakeMap {
                entries: 1,
                discard: false,
            });

            let module_key = c.key(MODULE);

            c.emit_internal(Op::AssignLocal {
                key: module_key,
                cache: EntryCache::default(),
            });
            c.body(c.graph.module(module).ast.body())?;

            let name = c.key(MODULE);

            c.emit_internal(Op::GetVariable {
                key: name,
                invoke: false,
                discard: false,
                cache: LookupCache::default(),
            });
            c.emit_internal(Op::Push(EXPORTS.into()));
            c.emit_internal(Op::GetProperty {
                call: None,
                discard: false,
            });

            Ok(())
        });

        self.modules.pop();

        let slot = ImportSlot(u32::try_from(self.imports.len()).expect("fewer than 2^32 imports"));
        let import = (slot, code?);

        self.imports.insert(module, import);

        Ok(import)
    }

    pub(crate) fn environment(&self, name: &str) -> Value {
        self.options
            .environment
            .get(name)
            .map_or(Value::Null, |value| Value::from_string(value.as_str()))
    }

    pub(crate) fn file_name(path: &str) -> Rc<str> {
        Path::new(path)
            .file_name()
            .map_or(path.into(), |name| name.to_string_lossy().into())
    }

    pub(crate) fn debug(&self) -> bool {
        self.options.debug
    }

    pub(crate) fn unexpected_node_error(&self, id: NodeId) -> CompileError {
        CompileError::UnexpectedNode {
            kind: self.kind(id).as_node().type_name(),
            location: self.location(id),
        }
    }
}
