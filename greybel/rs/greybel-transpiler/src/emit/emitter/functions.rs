use greybel_core::syntax::{ArrowBody, Function, Identifier, Return};
use greybel_core::{BinaryOp, NodeId, NodeKind, UnaryOp};
use greybel_symbols::literals::NULL;
use greybel_symbols::{Keyword, Punct};

use super::{EmitFeatures, Emitter};
use crate::emit::{EmitFlags, EmitState};
use crate::error::Result;

pub(crate) trait EmitFunctions: Emitter {
    fn emit_deferred_functions(
        &mut self,
        s: &mut EmitState<'_>,
        functions: &[NodeId],
    ) -> Result<()> {
        for &function in functions {
            let NodeKind::Function(node) = s.kind(function) else {
                continue;
            };

            s.eol();
            self.emit_function_body(s, node)?;
            s.keyword(Keyword::EndFunction);
        }

        Ok(())
    }

    fn emit_function_body(&mut self, s: &mut EmitState<'_>, node: &Function) -> Result<()> {
        self.emit_expression_defaults(s, node)?;

        self.emit_body(s, s.ast().list(node.body))
    }

    fn emit_expression_defaults(&mut self, s: &mut EmitState<'_>, node: &Function) -> Result<()> {
        for &parameter in s.ast().list(node.parameters) {
            let Some(assignment) = s.expression_default(parameter) else {
                continue;
            };

            self.begin_line(s);
            s.keyword(Keyword::If);
            s.push(" ");
            s.push(UnaryOp::AddressOf.as_str());
            self.emit(s, assignment.variable, EmitFlags::default())?;
            s.push_fmt(format_args!(
                "{0}{1}{0}{NULL} ",
                Self::INFIX_PADDING,
                BinaryOp::Equal.as_str()
            ));
            s.keyword(Keyword::Then);
            s.push(" ");
            self.emit(s, assignment.variable, EmitFlags::default())?;
            s.push_fmt(format_args!(
                "{0}{1}{0}",
                Self::INFIX_PADDING,
                Punct::Assign
            ));
            self.emit(s, assignment.init, EmitFlags::default())?;
            s.eol();
        }

        Ok(())
    }

    fn emit_name(
        &mut self,
        s: &mut EmitState<'_>,
        id: NodeId,
        node: &Identifier,
        flags: EmitFlags,
    ) -> Result<()> {
        self.emit_identifier(s, node, flags)?;
        self.emit_written_type(s, id);

        Ok(())
    }

    fn emit_templates(&mut self, s: &mut EmitState<'_>, function: NodeId) {
        if let Some(names) = s.written_templates(function) {
            s.push_fmt(format_args!("<{names}>"));
        }
    }

    fn emit_return_type(&mut self, s: &mut EmitState<'_>, function: NodeId) {
        if let Some(ty) = s.written_returns(function) {
            s.push_fmt(format_args!(": {ty}"));
        }
    }

    fn emit_arrow(
        &mut self,
        s: &mut EmitState<'_>,
        node: &Function,
        flags: EmitFlags,
    ) -> Result<()> {
        s.punct(Punct::LParen);

        for (index, &parameter) in s.ast().list(node.parameters).to_vec().iter().enumerate() {
            if index > 0 {
                s.push(Self::ARGUMENT_SEPARATOR);
            }

            self.emit(s, parameter, EmitFlags::default())?;
        }

        s.punct(Punct::RParen);
        s.push(" ");
        s.punct(Punct::Arrow);

        if node.arrow == Some(ArrowBody::Expression) {
            let body = s.ast().list(node.body);

            if let Some(&statement) = body.first()
                && let NodeKind::Return(Return {
                    argument: Some(argument),
                }) = s.kind(statement)
            {
                let argument = *argument;

                s.push(" ");
                self.emit(s, argument, EmitFlags::default())?;
            }

            return Ok(());
        }

        if flags.header_only {
            return Ok(());
        }

        self.emit_arrow_body(s, node)
    }
}

impl<E: Emitter> EmitFunctions for E {}
