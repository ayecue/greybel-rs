use std::borrow::Cow;
use std::cell::OnceCell;

use greybel_core::ast::NodeList;
use greybel_core::syntax::greybel::{Class, ClassMember};
use greybel_core::syntax::{Literal, Member};
use greybel_core::{Ast, Atom, NodeId, NodeKind, Span, TextSpan};
use greybel_documents::{Module, ModuleGraph, ModuleId};
use greybel_symbols::delimiters::QUOTE;
use greybel_symbols::names::UNNAMED_SOURCE;
use greybel_symbols::{Keyword, Punct};
use rustc_hash::{FxHashMap, FxHashSet};

use super::operands::OperandTable;
use super::{ConstantTable, EmitOutput, LineBreakStyle, LiteralStyle, LiteralText, Origin};
use crate::options::TranspileOptions;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)]
pub struct EmitFlags {
    pub header_only: bool,
    pub is_command: bool,
    pub is_member: bool,
    pub base_is_scope: bool,
    pub is_for_loop_variable: bool,
    pub logical_continuation: bool,
}

pub struct EmitState<'a> {
    ast: &'a Ast,
    graph: Option<&'a ModuleGraph>,
    namespaces: &'a [String],
    module: Option<ModuleId>,
    output_module: Option<ModuleId>,
    options: &'a TranspileOptions,
    output: EmitOutput,
    emitted_native_imports: FxHashSet<ModuleId>,
    class_members: FxHashMap<NodeId, ClassMember>,
    self_returns: FxHashSet<NodeId>,
    constructors: FxHashMap<NodeId, NodeList>,
    constants: Option<&'a ConstantTable>,
    variable_constants: FxHashSet<String>,
    assignment_targets: FxHashSet<NodeId>,
    operands: OnceCell<OperandTable>,
}

impl<'a> EmitState<'a> {
    #[must_use]
    pub fn new(ast: &'a Ast, options: &'a TranspileOptions) -> Self {
        Self {
            ast,
            graph: None,
            namespaces: &[],
            module: None,
            output_module: None,
            options,
            output: EmitOutput::with_capacity(ast.source().len()),
            emitted_native_imports: FxHashSet::default(),
            class_members: FxHashMap::default(),
            self_returns: FxHashSet::default(),
            constructors: FxHashMap::default(),
            constants: None,
            variable_constants: FxHashSet::default(),
            assignment_targets: FxHashSet::default(),
            operands: OnceCell::new(),
        }
    }

    #[must_use]
    pub fn for_module(
        graph: &'a ModuleGraph,
        namespaces: &'a [String],
        module: ModuleId,
        options: &'a TranspileOptions,
    ) -> Self {
        Self {
            graph: Some(graph),
            namespaces,
            module: Some(module),
            output_module: Some(module),
            ..Self::new(&graph.module(module).ast, options)
        }
    }

    #[must_use]
    pub fn ast(&self) -> &'a Ast {
        self.ast
    }

    pub fn enter_class(&mut self, class: &Class) {
        for member in &class.members {
            self.class_members.insert(member.assignment, *member);
        }

        if let Some(constructor) = &class.constructor {
            self.self_returns
                .extend(constructor.self_returns.iter().copied());
            self.constructors
                .insert(constructor.function, constructor.body);
        }
    }

    #[must_use]
    pub fn class_member(&self, id: NodeId) -> Option<ClassMember> {
        self.class_members.get(&id).copied()
    }

    #[must_use]
    pub fn written_body(&self, function: NodeId) -> Option<NodeList> {
        self.constructors.get(&function).copied()
    }

    #[must_use]
    pub fn written_type(&self, id: NodeId) -> Option<&'a str> {
        if !self.options.dev_mode {
            return None;
        }

        let types = &self.ast.type_syntax;
        let span = types.parameter(id).or_else(|| types.variable(id))?;

        Some(self.ast.slice(span))
    }

    #[must_use]
    pub fn written_returns(&self, function: NodeId) -> Option<&'a str> {
        if !self.options.dev_mode {
            return None;
        }

        Some(self.ast.slice(self.ast.type_syntax.returns(function)?))
    }

    #[must_use]
    pub fn written_templates(&self, function: NodeId) -> Option<String> {
        let names = self.ast.type_syntax.templates(function);

        if !self.options.dev_mode || names.is_empty() {
            return None;
        }

        let names: Vec<&str> = names.iter().map(|&name| self.ast.slice(name)).collect();

        Some(names.join(", "))
    }

    #[must_use]
    pub fn with_constants(mut self, constants: &'a ConstantTable) -> Self {
        if constants.is_empty() || self.options.dev_mode {
            return self;
        }

        self.variable_constants = constants.variable_names_in(self.ast);
        self.assignment_targets = self
            .ast
            .nodes()
            .iter()
            .filter_map(|(_, node)| match &node.kind {
                NodeKind::Assignment(assignment) => Some(assignment.variable),
                _ => None,
            })
            .collect();
        self.constants = Some(constants);

        self
    }

    #[must_use]
    pub fn constant(&self, member: NodeId, node: &Member) -> Option<&'a str> {
        let constants = self.constants?;

        if self.assignment_targets.contains(&member) {
            return None;
        }

        let NodeKind::Identifier(name) = self.ast.kind(node.base) else {
            return None;
        };
        let name = self.ast.resolve(name.name);

        if self.variable_constants.contains(name) {
            return None;
        }

        constants.get(name, self.ast.text(node.identifier)?)
    }

    #[must_use]
    pub fn is_self_return(&self, id: NodeId) -> bool {
        self.self_returns.contains(&id)
    }

    #[must_use]
    pub fn is_shortcut(&self, id: NodeId, shortcut: bool) -> bool {
        shortcut && (self.options.dev_mode || !self.ast.expansions.is_block(id))
    }

    #[must_use]
    pub fn options(&self) -> &'a TranspileOptions {
        self.options
    }

    #[must_use]
    pub fn kind(&self, id: NodeId) -> &'a NodeKind {
        self.ast.kind(id)
    }

    #[must_use]
    pub fn comments(&self, id: NodeId) -> Option<&'a greybel_core::NodeCommentContainer> {
        self.ast.node_comments(id)
    }

    #[must_use]
    pub fn read(&self, text: TextSpan) -> Cow<'a, str> {
        self.ast.read(text)
    }

    #[must_use]
    pub fn slice(&self, span: Span) -> &'a str {
        self.ast.slice(span)
    }

    #[must_use]
    pub fn is_omitted(&self, id: NodeId) -> bool {
        !self.options.dev_mode && matches!(self.ast.kind(id), NodeKind::TypeStatement(_))
    }

    #[must_use]
    pub fn is_noop(&self, id: NodeId) -> bool {
        matches!(self.ast.kind(id), NodeKind::Noop(_))
    }

    #[must_use]
    pub fn resolve(&self, atom: Atom) -> &'a str {
        self.ast.resolve(atom)
    }

    #[must_use]
    pub fn text(&self, id: NodeId) -> &'a str {
        self.ast.text(id).unwrap_or_default()
    }

    pub fn mark(&mut self, id: NodeId) {
        if let Some(module) = self.module {
            let line = self.start_line(id);

            self.output.mark(Origin { module, line });
        }
    }

    #[must_use]
    pub fn start_line(&self, id: NodeId) -> u32 {
        self.ast.lines().line(self.ast.span(id).start)
    }

    #[must_use]
    pub fn end_line(&self, id: NodeId) -> u32 {
        let span = self.ast.span(id);

        self.ast
            .lines()
            .line(span.end.saturating_sub(1).max(span.start))
    }

    #[must_use]
    pub fn source_label(&self) -> String {
        match (self.graph, self.module) {
            (Some(graph), Some(module)) => graph.module(module).uri.label(),
            _ => UNNAMED_SOURCE.to_owned(),
        }
    }

    #[must_use]
    pub fn module(&self) -> Option<&'a Module> {
        Some(self.graph?.module(self.module?))
    }

    #[must_use]
    pub fn graph(&self) -> Option<&'a ModuleGraph> {
        self.graph
    }

    #[must_use]
    pub fn namespace(&self, module: ModuleId) -> &'a str {
        &self.namespaces[module.index()]
    }

    #[must_use]
    pub fn output_module(&self) -> Option<ModuleId> {
        self.output_module
    }

    pub fn emitted_native_imports_mut(&mut self) -> &mut FxHashSet<ModuleId> {
        &mut self.emitted_native_imports
    }

    pub fn with_module<T>(&mut self, module: ModuleId, f: impl FnOnce(&mut Self) -> T) -> T {
        let graph = self.graph.expect("dependencies only exist when bundling");
        let previous = (self.ast, self.module);

        self.ast = &graph.module(module).ast;
        self.module = Some(module);

        let result = f(self);

        (self.ast, self.module) = previous;

        result
    }

    pub fn push(&mut self, text: &str) {
        self.output.push(text);
    }

    pub fn push_fmt(&mut self, args: std::fmt::Arguments<'_>) {
        self.output.push_fmt(args);
    }

    pub fn literal(
        &mut self,
        id: NodeId,
        literal: &Literal,
        source: &str,
        style: LiteralStyle,
        helper: &str,
    ) {
        let text =
            LiteralText::new(literal, source, style).with_line_breaks(self.line_breaks(helper));

        if let Some(raw_text) = text.raw_text() {
            self.push(raw_text);
        } else if text.concatenates() && self.is_concatenation_operand(id) {
            self.push_fmt(format_args!("({text})"));
        } else {
            self.push_fmt(format_args!("{text}"));
        }
    }

    #[must_use]
    pub(crate) fn is_concatenation_operand(&self, id: NodeId) -> bool {
        self.operands
            .get_or_init(|| OperandTable::of(self.ast))
            .contains(id)
    }

    #[must_use]
    pub fn line_breaks<'f>(&self, helper: &'f str) -> LineBreakStyle<'f> {
        LineBreakStyle::of(self.options, helper)
    }

    pub fn keyword(&mut self, keyword: Keyword) {
        self.output.push(keyword.as_str());
    }

    pub fn punct(&mut self, punct: Punct) {
        self.output.push(punct.as_str());
    }

    pub fn write_quoted_text(&mut self, text: &str) {
        self.output.push(QUOTE);
        self.output.push(text);
        self.output.push(QUOTE);
    }

    pub fn eol(&mut self) {
        self.output.eol();
    }

    #[must_use]
    pub fn is_line_empty(&self) -> bool {
        self.output.is_line_empty()
    }

    #[must_use]
    pub fn line_count(&self) -> usize {
        self.output.line_count()
    }

    pub fn append_comment(&mut self, index: Option<usize>, comment: impl std::fmt::Display) {
        let index = index.unwrap_or_else(|| self.output.line_count());

        self.output.append_comment(index, comment);
    }

    #[must_use]
    pub fn into_output(self) -> EmitOutput {
        self.output
    }

    #[must_use]
    pub fn with_emitted_native_imports(mut self, native_imports: FxHashSet<ModuleId>) -> Self {
        self.emitted_native_imports = native_imports;

        self
    }

    #[must_use]
    pub fn into_parts(self) -> (EmitOutput, FxHashSet<ModuleId>) {
        (self.output, self.emitted_native_imports)
    }
}
