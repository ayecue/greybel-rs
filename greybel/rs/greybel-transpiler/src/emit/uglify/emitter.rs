use greybel_core::NodeId;
use greybel_core::syntax::{Call, Function, Identifier, IdentifierKind, Literal, Member};
use greybel_symbols::delimiters::QUOTE;
use greybel_symbols::names::{LOOP_INDEX_PREFIX, LOOP_INDEX_SUFFIX};
use greybel_symbols::{Keyword, Punct};

use super::{UglifyEmitter, member_flags};
use crate::bundle::multiline_string_helper;
use crate::emit::{EmitExpressions, EmitFlags, EmitFunctions, EmitState, Emitter, LiteralStyle};
use crate::error::Result;

impl Emitter for UglifyEmitter {
    fn preamble(&self, s: &EmitState<'_>) -> Vec<String> {
        self.preamble_lines(s)
    }

    fn runtime_name<'a>(&'a self, name: &'a str) -> &'a str {
        if self.options.disable_namespaces_optimization {
            name
        } else {
            self.minification.rename(name)
        }
    }

    fn end_statement(&mut self, s: &mut EmitState<'_>) {
        if !s.is_line_empty() {
            s.eol();
        }
    }

    fn emit_parameters(&mut self, s: &mut EmitState<'_>, parameters: &[NodeId]) -> Result<()> {
        let parameters = s.emitted_parameters(parameters);

        self.in_parameters(s, |this, s| {
            this.emit_with_separators(s, &parameters, Punct::Comma.as_str())
        })
    }

    fn emit_function(
        &mut self,
        s: &mut EmitState<'_>,
        id: NodeId,
        node: &Function,
        flags: EmitFlags,
    ) -> Result<()> {
        s.keyword(Keyword::Function);
        self.emit_templates(s, id);

        if !node.parameters.is_empty() || s.written_returns(id).is_some() {
            s.punct(Punct::LParen);
            self.emit_parameters(s, s.ast().list(node.parameters))?;
            s.punct(Punct::RParen);
        }

        self.emit_return_type(s, id);

        if flags.header_only {
            return Ok(());
        }

        s.eol();
        self.emit_function_body(s, node)?;
        s.keyword(Keyword::EndFunction);

        Ok(())
    }

    fn emit_identifier(
        &mut self,
        s: &mut EmitState<'_>,
        node: &Identifier,
        flags: EmitFlags,
    ) -> Result<()> {
        let original = s.resolve(node.name);

        if self.options.disable_namespaces_optimization {
            s.push(original);

            return Ok(());
        }

        if !flags.is_member
            && node.kind == IdentifierKind::Variable
            && let Some(alias) = self.minification.native_alias(original)
        {
            s.push(alias);

            return Ok(());
        }

        let name = if !flags.is_member || flags.base_is_scope {
            self.resolved_name(original)
        } else {
            original
        };

        s.push(name);
        if flags.is_for_loop_variable && name != original {
            let index_name = format!("{LOOP_INDEX_PREFIX}{name}{LOOP_INDEX_SUFFIX}");

            self.rename_loop_index(original, index_name);
        }

        Ok(())
    }

    fn emit_member(&mut self, s: &mut EmitState<'_>, node: &Member) -> Result<()> {
        self.emit(s, node.base, EmitFlags::default())?;
        s.punct(Punct::Dot);

        let flags = member_flags(s, node.base);

        self.emit(s, node.identifier, flags)
    }

    fn emit_call(
        &mut self,
        s: &mut EmitState<'_>,
        id: NodeId,
        node: &Call,
        _flags: EmitFlags,
    ) -> Result<()> {
        self.emit(s, node.base, EmitFlags::default())?;

        let Some(argument) = s.has_index_key_argument(node) else {
            return self.emit_call_arguments(s, id, node);
        };

        if let Some(name) = s.string_value(argument) {
            let name = self.minification.rename(&name);

            s.punct(Punct::LParen);
            s.write_quoted_text(name);
            s.punct(Punct::RParen);

            return Ok(());
        }

        s.punct(Punct::LParen);
        self.emit(s, argument, EmitFlags::default())?;
        s.punct(Punct::RParen);

        Ok(())
    }

    fn emit_literal(&mut self, s: &mut EmitState<'_>, id: NodeId, node: &Literal) -> Result<()> {
        let source = s.text(id);

        if !self.options.disable_literals_optimization
            && !self.in_parameters
            && let Some(variable) = self.minification.hoisted_variable(node, source)
        {
            s.push(variable);

            return Ok(());
        }

        s.literal(
            id,
            node,
            source,
            LiteralStyle::Output,
            self.runtime_name(multiline_string_helper(s.options())),
        );

        Ok(())
    }

    fn require_assignment(&self, _s: &EmitState<'_>, namespace: &str) -> String {
        format!(
            "{}{}{}{QUOTE}{namespace}{QUOTE}{}",
            Punct::Assign,
            self.require_name(),
            Punct::LParen,
            Punct::RParen
        )
    }
}
