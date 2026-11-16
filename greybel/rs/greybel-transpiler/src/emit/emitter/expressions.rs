use greybel_core::syntax::greybel::{Bitwise, BitwiseOp};
use greybel_core::syntax::{Call, Index, MapEntry, Member, Unary};
use greybel_core::{Dialect, NodeId, UnaryOp};
use greybel_symbols::names::{BIT_AND, BIT_OR, BITWISE};
use greybel_symbols::{Keyword, Punct};

use super::{EmitFunctions, Emitter};
use crate::emit::{EmitFlags, EmitState};
use crate::error::{Result, TranspileError};

pub(crate) trait EmitExpressions: Emitter {
    fn emit_bitwise(&mut self, s: &mut EmitState<'_>, node: &Bitwise) -> Result<()> {
        let Bitwise {
            operator,
            left,
            right,
        } = *node;

        if s.options().dialect == Dialect::GreyScript {
            if s.options().dev_mode {
                self.emit(s, left, EmitFlags::default())?;
                s.push_fmt(format_args!(
                    "{0}{1}{0}",
                    Self::INFIX_PADDING,
                    operator.as_str()
                ));

                return self.emit(s, right, EmitFlags::default());
            }

            s.push(BITWISE);
            s.punct(Punct::LParen);
            s.write_quoted_text(operator.as_str());
            s.push(Self::ARGUMENT_SEPARATOR);
        } else {
            let function = match operator {
                BitwiseOp::Or => BIT_OR,
                BitwiseOp::And => BIT_AND,
                BitwiseOp::ShiftLeft | BitwiseOp::ShiftRight | BitwiseOp::UnsignedShiftRight => {
                    return Err(TranspileError::UnsupportedOperator {
                        file: s.source_label(),
                        operator: operator.as_str(),
                    });
                }
            };

            s.push(function);
            s.punct(Punct::LParen);
        }

        self.emit(s, left, EmitFlags::default())?;
        s.push(Self::ARGUMENT_SEPARATOR);
        self.emit(s, right, EmitFlags::default())?;
        s.punct(Punct::RParen);

        Ok(())
    }

    fn emit_member_or_constant(
        &mut self,
        s: &mut EmitState<'_>,
        id: NodeId,
        node: &Member,
    ) -> Result<()> {
        match s.constant(id, node) {
            Some(value) => {
                s.push(value);

                Ok(())
            }
            None => self.emit_member(s, node),
        }
    }

    fn emit_index(&mut self, s: &mut EmitState<'_>, node: &Index) -> Result<()> {
        self.emit(s, node.base, EmitFlags::default())?;
        s.punct(Punct::LBracket);
        self.emit(s, node.index, EmitFlags::default())?;
        s.punct(Punct::RBracket);

        Ok(())
    }

    fn emit_call_arguments(
        &mut self,
        s: &mut EmitState<'_>,
        id: NodeId,
        node: &Call,
    ) -> Result<()> {
        if node.arguments.is_empty() {
            return Ok(());
        }

        s.punct(Punct::LParen);

        let deferred_functions = self.emit_items(
            s,
            id,
            s.ast().list(node.arguments),
            Self::ARGUMENT_SEPARATOR,
        )?;

        s.punct(Punct::RParen);

        self.emit_deferred_functions(s, &deferred_functions)
    }

    fn emit_unary(&mut self, s: &mut EmitState<'_>, node: &Unary) -> Result<()> {
        match node.operator {
            UnaryOp::Not => {
                s.keyword(Keyword::Not);
                s.push(" ");
            }
            UnaryOp::New => {
                s.keyword(Keyword::New);
                s.push(" ");
            }
            operator => s.push(operator.as_str()),
        }

        self.emit(s, node.argument, EmitFlags::default())
    }

    fn emit_map_key(&mut self, s: &mut EmitState<'_>, node: &MapEntry) -> Result<()> {
        if node.shorthand {
            let name = s.text(node.key).to_owned();

            s.write_quoted_text(&name);

            return Ok(());
        }

        self.emit(s, node.key, EmitFlags::default())
    }

    fn emit_items(
        &mut self,
        s: &mut EmitState<'_>,
        container: NodeId,
        items: &[NodeId],
        separator: &str,
    ) -> Result<Vec<NodeId>> {
        let mut deferred_functions = Vec::new();

        for (index, &item) in items.iter().enumerate() {
            if index > 0 {
                s.push(separator);
            }

            match s.deferred_function(item, container) {
                Some(function) => {
                    deferred_functions.push(function);

                    let header = EmitFlags {
                        header_only: true,
                        ..EmitFlags::default()
                    };

                    self.emit(s, item, header)?;
                }
                None => self.emit(s, item, EmitFlags::default())?,
            }
        }

        deferred_functions.sort_by_key(|&function| s.end_line(function));

        Ok(deferred_functions)
    }

    fn emit_with_separators(
        &mut self,
        s: &mut EmitState<'_>,
        items: &[NodeId],
        separator: &str,
    ) -> Result<()> {
        for (index, &item) in items.iter().enumerate() {
            if index > 0 {
                s.push(separator);
            }

            self.emit(s, item, EmitFlags::default())?;
        }

        Ok(())
    }
}

impl<E: Emitter> EmitExpressions for E {}
