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
    pub return_types: bool,
}

impl Default for InlayHintOptions {
    fn default() -> Self {
        Self {
            variable_types: true,
            parameter_names: true,
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

    fn visit(&mut self, node: NodeId) {
        let span = self.ast.span(node);

        if span.end < self.span.start || span.start > self.span.end {
            return;
        }

        if span.start >= self.span.start {
            match self.ast.kind(node) {
                NodeKind::Assignment(assignment) if self.options.variable_types => {
                    self.variable_type(assignment.variable, assignment.init);
                }
                NodeKind::Call(call) if self.options.parameter_names => {
                    self.parameter_names(call.base, self.ast.list(call.arguments));
                }
                NodeKind::Function(_) if self.options.return_types => self.return_type(node),
                _ => {}
            }
        }

        self.ast.for_each_child(node, |child| self.visit(child));
    }

    fn variable_type(&mut self, variable: NodeId, init: NodeId) {
        let ast = self.ast;

        if !matches!(ast.kind(variable), NodeKind::Identifier(_))
            || matches!(ast.kind(init), NodeKind::Literal(_) | NodeKind::Function(_))
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

        if signature.parameters.len() < 2 || arguments.len() < 2 {
            return;
        }

        for (&argument, parameter) in arguments.iter().zip(&signature.parameters) {
            if Self::is_literal(ast, argument) {
                self.hints.push(InlayHint {
                    position: self.position(Self::argument_start(ast, argument)),
                    label: format!("{}:", parameter.name),
                    kind: InlayHintKind::Parameter,
                });
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
        let after = source.strip_prefix("function")?;
        let keyword_end = span.start + 8;

        if !after.trim_start_matches([' ', '\t']).starts_with('(') {
            return Some(keyword_end);
        }

        let mut depth = 0_u32;
        let mut quoted = false;

        for (index, character) in after.char_indices() {
            match character {
                '"' => quoted = !quoted,
                '(' if !quoted => depth += 1,
                ')' if !quoted => {
                    depth -= 1;

                    if depth == 0 {
                        let index = u32::try_from(index).ok()?;

                        return Some(keyword_end + index + 1);
                    }
                }
                '\n' => return None,
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

    fn is_literal(ast: &Ast, node: NodeId) -> bool {
        match ast.kind(node) {
            NodeKind::Literal(_) | NodeKind::List(_) | NodeKind::Map(_) => true,
            NodeKind::Unary(unary) => matches!(ast.kind(unary.argument), NodeKind::Literal(_)),
            _ => false,
        }
    }

    fn push_type(&mut self, offset: u32, ty: TypeId) {
        if ty == TypeId::UNKNOWN || ty == TypeId::ANY {
            return;
        }

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
