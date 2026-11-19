use greybel_core::{Ast, ColumnSettings, NodeId, NodeKind, Position, Span};

use crate::analysis::Analysis;
use crate::model::{DocumentIndex, Site};
use crate::types::TypeId;

const MAX_LABEL: usize = 60;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InlayHint {
    pub position: Position,
    pub label: String,
    pub kind: InlayHintKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum InlayHintKind {
    Type,
    Parameter,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InlayHintOptions {
    pub variable_types: bool,
    pub parameter_names: bool,
    pub parameter_types: bool,
    pub return_types: bool,
}

impl Default for InlayHintOptions {
    fn default() -> Self {
        Self {
            variable_types: true,
            parameter_names: true,
            parameter_types: true,
            return_types: true,
        }
    }
}

pub(super) struct InlayHintCollector<'a> {
    ast: &'a Ast,
    analysis: &'a Analysis,
    document: DocumentIndex,
    options: InlayHintOptions,
    span: Span,
    hints: Vec<InlayHint>,
}

impl<'a> InlayHintCollector<'a> {
    pub(super) fn collect(
        ast: &'a Ast,
        analysis: &'a Analysis,
        document: DocumentIndex,
        options: InlayHintOptions,
        span: Span,
    ) -> Vec<InlayHint> {
        let mut hints = Self {
            ast,
            analysis,
            document,
            options,
            span,
            hints: Vec::new(),
        };

        hints.visit(ast.root());
        hints.hints.sort_by_key(|hint| hint.position);

        hints.hints
    }

    fn visit(&mut self, root: NodeId) {
        let mut nodes = self.ast.descendants(root);

        while let Some(node) = nodes.next() {
            let span = self.ast.span(node);

            if span.end < self.span.start || span.start > self.span.end {
                nodes.skip_children();

                continue;
            }

            if span.start >= self.span.start {
                match self.ast.kind(node) {
                    NodeKind::Assignment(assignment) if self.options.variable_types => {
                        self.variable_type(assignment.variable, assignment.init);
                    }
                    NodeKind::Call(call) if self.options.parameter_names => {
                        self.parameter_names(call.base, self.ast.list(call.arguments));
                    }
                    NodeKind::Function(_) => {
                        if self.options.parameter_types {
                            self.parameter_types(node);
                        }

                        if self.options.return_types {
                            self.return_type(node);
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    fn variable_type(&mut self, variable: NodeId, init: NodeId) {
        let ast = self.ast;

        if !matches!(ast.kind(variable), NodeKind::Identifier(_))
            || matches!(ast.kind(init), NodeKind::Literal(_) | NodeKind::Function(_))
            || ast.type_syntax.variable(variable).is_some()
        {
            return;
        }

        let site = self.site(variable);
        let &[slot] = self.analysis.references().slots(site) else {
            return;
        };
        let first = self
            .analysis
            .references()
            .definition_sites(slot)
            .iter()
            .filter(|definition| definition.document == self.document)
            .min_by_key(|definition| ast.span(definition.node).start);

        if first != Some(&site) {
            return;
        }

        let ty = self
            .analysis
            .node_type(self.document, variable)
            .unwrap_or_else(|| self.analysis.model().slot_type(slot));

        self.push_type(ast.span(variable).end, ty);
    }

    fn parameter_names(&mut self, callee: NodeId, arguments: &[NodeId]) {
        let ast = self.ast;
        let Some(ty) = self.analysis.node_type(self.document, callee) else {
            return;
        };
        let Some(function) = self.analysis.types().function_refs(ty).next() else {
            return;
        };
        let method = matches!(ast.kind(callee), NodeKind::Member(_));
        let Some(signature) = self.analysis.model().signature(function, method) else {
            return;
        };

        for (&argument, parameter) in arguments.iter().zip(&signature.parameters) {
            if Self::has_name(ast, argument, parameter.name) {
                continue;
            }

            self.hints.push(InlayHint {
                position: self.position(Self::argument_start(ast, argument)),
                label: format!("{}:", parameter.name),
                kind: InlayHintKind::Parameter,
            });
        }
    }

    fn parameter_types(&mut self, node: NodeId) {
        let ast = self.ast;
        let model = self.analysis.model();
        let Some(id) = model.function_at(self.site(node)) else {
            return;
        };
        let data = model.function(id);

        for parameter in &data.parameters {
            let doc = data.doc.as_ref().and_then(|doc| doc.param(&parameter.name));

            if doc.is_some_and(|doc| doc.ty.is_some()) {
                continue;
            }

            let name = match ast.kind(parameter.node) {
                NodeKind::Assignment(assignment) => assignment.variable,
                _ => parameter.node,
            };
            let ty = model.slot_type(parameter.slot);

            if ty != TypeId::UNKNOWN {
                self.push_label(ast.span(name).end, ty);
            }
        }
    }

    fn return_type(&mut self, node: NodeId) {
        let model = self.analysis.model();
        let Some(id) = model.function_at(self.site(node)) else {
            return;
        };
        let data = model.function(id);

        if data.doc.as_ref().is_some_and(|doc| doc.returns.is_some()) {
            return;
        }

        let Some(end) = self.header_end(node) else {
            return;
        };

        self.push_type(end, model.slot_type(data.return_slot));
    }

    fn header_end(&self, node: NodeId) -> Option<u32> {
        let span = self.ast.span(node);
        let source = self.ast.slice(span);
        let (offset, header) = match source.strip_prefix("function") {
            Some(header) => (8, header),
            None => (0, source),
        };
        let mut end = offset;
        let mut parentheses = 0_u32;
        let mut angles = 0_u32;
        let mut quoted = false;

        for (index, character) in header.char_indices() {
            let index = u32::try_from(index).ok()?;

            match character {
                '"' if parentheses > 0 => quoted = !quoted,
                _ if quoted => {}
                '(' if angles == 0 => parentheses += 1,
                ')' if parentheses > 0 => {
                    parentheses -= 1;

                    if parentheses == 0 {
                        return Some(span.start + offset + index + 1);
                    }
                }
                '<' if parentheses == 0 => angles += 1,
                '>' if parentheses == 0 && angles > 0 => {
                    angles -= 1;

                    if angles == 0 {
                        end = offset + index + 1;
                    }
                }
                ' ' | '\t' => {}
                _ if parentheses == 0 && angles == 0 => {
                    return (offset > 0).then_some(span.start + end);
                }
                _ => {}
            }
        }

        None
    }

    fn argument_start(ast: &Ast, node: NodeId) -> u32 {
        let start = ast.span(node).start;
        let negated = matches!(ast.kind(node), NodeKind::Literal(literal) if literal.negated);
        let before = ast.source()[..start as usize].trim_end();

        match before.strip_suffix('-') {
            Some(prefix) if negated => u32::try_from(prefix.len()).unwrap_or(start),
            _ => start,
        }
    }

    fn has_name(ast: &Ast, node: NodeId, name: &str) -> bool {
        let identifier = match ast.kind(node) {
            NodeKind::Identifier(_) => node,
            NodeKind::Member(member) => member.identifier,
            _ => return false,
        };

        ast.slice(ast.span(identifier)).eq_ignore_ascii_case(name)
    }

    fn push_type(&mut self, offset: u32, ty: TypeId) {
        if ty != TypeId::UNKNOWN && ty != TypeId::ANY {
            self.push_label(offset, ty);
        }
    }

    fn push_label(&mut self, offset: u32, ty: TypeId) {
        let mut label = format!(
            ": {}",
            self.analysis.notation_in(self.analysis.types()).display(ty)
        );

        if label.len() > MAX_LABEL {
            let cut = (0..=MAX_LABEL - 3)
                .rev()
                .find(|&index| label.is_char_boundary(index))
                .unwrap_or(0);

            label.truncate(cut);
            label.push_str("...");
        }

        self.hints.push(InlayHint {
            position: self.position(offset),
            label,
            kind: InlayHintKind::Type,
        });
    }

    fn site(&self, node: NodeId) -> Site {
        Site {
            document: self.document,
            node,
        }
    }

    fn position(&self, offset: u32) -> Position {
        self.ast.lines().position(offset, ColumnSettings::default())
    }
}
