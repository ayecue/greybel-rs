use std::borrow::Cow;

use greybel_core::syntax::Literal;
use greybel_core::{Ast, Atom, NodeId, NodeKind, Span, TextSpan};
use greybel_symbols::delimiters::QUOTE;
use greybel_symbols::names::UNNAMED_SOURCE;
use greybel_symbols::{Keyword, Punct};
use rustc_hash::FxHashSet;

use super::{EmitOutput, LineBreakStyle, LiteralStyle, LiteralText, Origin};
use crate::documents::{Module, ModuleGraph, ModuleId};
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
    pub fn module_id(&self) -> Option<ModuleId> {
        self.module
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
        literal: &Literal,
        source: &str,
        style: LiteralStyle,
        multiline_string: &str,
    ) {
        let text = LiteralText::new(literal, source, style)
            .with_line_breaks(self.line_breaks(multiline_string));

        match text.raw_text() {
            Some(raw_text) => self.push(raw_text),
            None => self.push_fmt(format_args!("{text}")),
        }
    }

    #[must_use]
    pub fn line_breaks<'f>(&self, multiline_string: &'f str) -> LineBreakStyle<'f> {
        LineBreakStyle::of(self.options, multiline_string)
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
