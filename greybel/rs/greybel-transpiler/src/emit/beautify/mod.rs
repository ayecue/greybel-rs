mod comments;
mod emitter;
mod shape;

use greybel_core::{NodeId, NodeKind};
use greybel_symbols::{Keyword, Punct};

use super::{EmitFlags, EmitFunctions, EmitState, Emitter};
use crate::error::Result;
use crate::options::{BeautifyOptions, Indentation};

pub(super) const SHORTHAND_OPERATORS: &[greybel_core::BinaryOp] = &[
    greybel_core::BinaryOp::Add,
    greybel_core::BinaryOp::Subtract,
    greybel_core::BinaryOp::Multiply,
    greybel_core::BinaryOp::Divide,
    greybel_core::BinaryOp::Modulo,
    greybel_core::BinaryOp::Power,
];
pub(super) const MAX_INLINE_ARGUMENTS: usize = 3;
pub(super) const MAX_INLINE_OPERATIONS: usize = 2;

#[derive(Debug, Clone, Default)]
pub struct BeautifyEmitter {
    options: BeautifyOptions,
    indent: usize,
    single_line: bool,
}

impl BeautifyEmitter {
    #[must_use]
    pub fn new(options: BeautifyOptions) -> Self {
        Self {
            options,
            ..Self::default()
        }
    }

    fn indentation(&self) -> Indent {
        match self.options.indentation {
            Indentation::Tab => Indent('\t', self.indent),
            Indentation::Spaces(width) => Indent(' ', width * self.indent),
        }
    }

    fn push_indent(&self, s: &mut EmitState<'_>) {
        s.push_fmt(format_args!("{}", self.indentation()));
    }

    fn single_line(
        &mut self,
        s: &mut EmitState<'_>,
        f: impl FnOnce(&mut Self, &mut EmitState<'_>) -> Result<()>,
    ) -> Result<()> {
        let previous = std::mem::replace(&mut self.single_line, true);
        let result = f(self, s);

        self.single_line = previous;

        result
    }

    fn with_indent(
        &mut self,
        s: &mut EmitState<'_>,
        f: impl FnOnce(&mut Self, &mut EmitState<'_>) -> Result<()>,
    ) -> Result<()> {
        self.indent += 1;

        let result = f(self, s);

        self.indent -= 1;

        result
    }

    fn emit_function_tail(&mut self, s: &mut EmitState<'_>, function: NodeId) -> Result<()> {
        let NodeKind::Function(node) = s.kind(function) else {
            return Ok(());
        };

        s.eol();
        self.with_indent(s, |this, s| this.emit_function_body(s, node))?;
        s.push_fmt(format_args!(
            "{}{}",
            self.indentation(),
            Keyword::EndFunction
        ));
        Self::emit_end_trailing_comments(s, function);

        Ok(())
    }

    fn emit_collection(
        &mut self,
        s: &mut EmitState<'_>,
        id: NodeId,
        fields: &[NodeId],
        (open, close): (&str, &str),
        sort_deferred: bool,
    ) -> Result<()> {
        match fields {
            [] => s.push_fmt(format_args!("{open}{close}")),
            &[field] => {
                let deferred_functions = s.deferred_function(field, id);

                s.push_fmt(format_args!("{open} "));

                let flags = EmitFlags {
                    header_only: deferred_functions.is_some(),
                    ..EmitFlags::default()
                };

                self.emit(s, field, flags)?;
                s.push_fmt(format_args!(" {close}"));
                Self::emit_trailing_comments(s, field, None);
                if let Some(function) = deferred_functions {
                    self.emit_function_tail(s, function)?;
                }
            }
            _ if !self.single_line => {
                self.with_indent(s, |this, s| {
                    s.push(open);
                    s.eol();
                    for &field in fields {
                        if s.is_noop(field) {
                            this.emit_blank_line(s, field);
                            continue;
                        }

                        this.emit_leading_comments(s, field);
                        this.push_indent(s);
                        this.emit(s, field, EmitFlags::default())?;
                        s.punct(Punct::Comma);
                        Self::emit_trailing_comments(s, field, None);
                        s.eol();
                    }

                    Ok(())
                })?;
                s.push_fmt(format_args!("{}{close}", self.indentation()));
            }
            _ => {
                s.push_fmt(format_args!("{open} "));

                let mut deferred_functions = Vec::new();
                let last = fields.len() - 1;

                for (index, &field) in fields.iter().enumerate() {
                    if s.is_noop(field) {
                        continue;
                    }

                    let function = s.deferred_function(field, id);

                    deferred_functions.extend(function);

                    let flags = EmitFlags {
                        header_only: function.is_some(),
                        ..EmitFlags::default()
                    };

                    self.emit(s, field, flags)?;
                    if index != last {
                        s.punct(Punct::Comma);
                        s.push(" ");
                    }
                }

                s.push_fmt(format_args!(" {close}"));
                if sort_deferred {
                    deferred_functions.sort_by_key(|&function| s.end_line(function));
                }

                for function in deferred_functions {
                    self.emit_function_tail(s, function)?;
                }
            }
        }

        Ok(())
    }

    fn emit_without_parens(&mut self, s: &mut EmitState<'_>, id: NodeId) -> Result<()> {
        self.emit(s, s.unwrap_from_parens(id), EmitFlags::default())
    }

    fn emit_block(&mut self, s: &mut EmitState<'_>, body: &[NodeId], end: Keyword) -> Result<()> {
        s.eol();
        self.with_indent(s, |this, s| this.emit_body(s, body))?;
        s.push_fmt(format_args!("{}{end}", self.indentation()));

        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
struct Indent(char, usize);

impl std::fmt::Display for Indent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        (0..self.1).try_for_each(|_| std::fmt::Write::write_char(f, self.0))
    }
}
