use greybel_core::lexer::NumericLiteral;
use greybel_core::syntax::greybel::{Include, Inject, Switch};
use greybel_core::syntax::{
    Assignment, Binary, Call, CallStatement, ComparisonGroup, ElseClause, For, Function, If,
    IfClause, Isa, List, ListItem, Literal, Logical, Map, MapEntry, Parenthesis, Slice, While,
};
use greybel_core::{IfClauseKind, LiteralValue, NodeId};
use greybel_symbols::contextual_keywords::{SWITCH, TYPE};
use greybel_symbols::delimiters::{
    ESCAPED_QUOTE, MAP_KEY_SEPARATOR, QUOTE, SLICE_SEPARATOR, STATEMENT_SEPARATOR,
};
use greybel_symbols::literals::NULL;
use greybel_symbols::{Keyword, Punct};

use super::{BeautifyEmitter, MAX_INLINE_ARGUMENTS, MAX_INLINE_OPERATIONS};
use crate::bundle::multiline_string_helper;
use crate::emit::{
    EmitExpressions, EmitFeatures, EmitFlags, EmitFunctions, EmitOutput, EmitState, EmitStatements,
    Emitter, LiteralStyle,
};
use crate::error::Result;

impl Emitter for BeautifyEmitter {
    const ARGUMENT_SEPARATOR: &'static str = ", ";
    const INFIX_PADDING: &'static str = " ";
    const RETAINS_COMMENTS: bool = true;

    fn separate_statements(&mut self, s: &mut EmitState<'_>) {
        s.eol();
        self.push_indent(s);
    }

    fn emit_switch(&mut self, s: &mut EmitState<'_>, node: &Switch) -> Result<()> {
        s.push(SWITCH);
        s.push(" ");
        self.emit_without_parens(s, node.subject)?;
        s.eol();
        self.with_indent(s, |this, s| {
            for case in &node.cases {
                this.push_indent(s);
                this.emit_switch_case(s, case)?;
                s.eol();
                this.with_indent(s, |this, s| this.emit_body(s, s.ast().list(case.body)))?;
            }

            if let Some(fallback) = node.fallback {
                this.push_indent(s);
                s.keyword(Keyword::Else);
                s.eol();
                this.with_indent(s, |this, s| this.emit_body(s, s.ast().list(fallback)))?;
            }

            Ok(())
        })?;
        s.push_fmt(format_args!("{}{}", self.indentation(), Keyword::EndSwitch));

        Ok(())
    }

    fn emit_block_body(
        &mut self,
        s: &mut EmitState<'_>,
        body: &[NodeId],
        end: Keyword,
    ) -> Result<()> {
        s.eol();
        self.with_indent(s, |this, s| this.emit_body(s, body))?;
        s.push_fmt(format_args!("{}{}", self.indentation(), end));

        Ok(())
    }

    fn emit_arrow_body(&mut self, s: &mut EmitState<'_>, node: &Function) -> Result<()> {
        s.eol();
        self.with_indent(s, |this, s| this.emit_function_body(s, node))?;
        s.push_fmt(format_args!(
            "{}{}",
            self.indentation(),
            Keyword::EndFunction
        ));

        Ok(())
    }

    /// Re-indents the lines after the first: the closing `end` line at the statement's depth, the others one deeper.
    fn emit_block_source(&mut self, s: &mut EmitState<'_>, id: NodeId) {
        let text = s.text(id).to_owned();
        let mut lines = text.lines();

        s.push(lines.next().unwrap_or_default().trim_end());

        for line in lines {
            let line = line.trim();

            s.eol();

            if line.is_empty() {
                continue;
            }

            if line.starts_with(Keyword::End.as_str()) {
                self.push_indent(s);
            } else {
                self.indent += 1;
                self.push_indent(s);
                self.indent -= 1;
            }

            s.push(line);
        }
    }

    fn finish(&self, output: EmitOutput) -> String {
        let mut text = String::with_capacity(output.text_len());

        for (index, (code, comment)) in output.lines().enumerate() {
            if index > 0 {
                text.push('\n');
            }

            let blank = code.trim().is_empty();

            if !blank {
                text.push_str(code);
            }

            if !comment.is_empty() {
                if !blank {
                    text.push(' ');
                }

                text.push_str(comment);
            }
        }

        text
    }

    fn emit_body(&mut self, s: &mut EmitState<'_>, body: &[NodeId]) -> Result<()> {
        for &item in body {
            if s.is_omitted(item) {
                continue;
            }

            if s.is_noop(item) {
                self.emit_blank_line(s, item);
                continue;
            }

            self.emit_leading_comments(s, item);
            self.push_indent(s);

            let first_line = s.line_count();
            let statement = EmitFlags {
                is_command: true,
                ..EmitFlags::default()
            };

            self.emit(s, item, statement)?;
            if s.line_count() > first_line && s.is_block_statement(item) {
                Self::emit_trailing_comments(s, item, Some(first_line));
                Self::emit_end_trailing_comments(s, item);
            } else {
                Self::emit_trailing_comments(s, item, None);
            }

            s.eol();
        }

        Ok(())
    }

    fn emit_parenthesis(
        &mut self,
        s: &mut EmitState<'_>,
        node: &Parenthesis,
        flags: EmitFlags,
    ) -> Result<()> {
        s.punct(Punct::LParen);

        let inner = EmitFlags {
            logical_continuation: flags.logical_continuation,
            ..EmitFlags::default()
        };

        self.emit(s, node.expression, inner)?;
        s.punct(Punct::RParen);

        Ok(())
    }

    fn emit_assignment(
        &mut self,
        s: &mut EmitState<'_>,
        _id: NodeId,
        node: &Assignment,
        _flags: EmitFlags,
    ) -> Result<()> {
        if s.options().dev_mode
            && let Some(declaration) = node.declaration
        {
            s.push(declaration.as_str());
            s.push(" ");
        }

        self.emit(s, node.variable, EmitFlags::default())?;
        if let Some(binary) = s.shorthand(node) {
            s.push_fmt(format_args!(
                " {}{} ",
                binary.operator.as_str(),
                Punct::Assign
            ));

            return self.emit_without_parens(s, binary.right);
        }

        s.push(" ");
        s.punct(Punct::Assign);
        s.push(" ");

        self.emit(s, node.init, EmitFlags::default())
    }

    fn emit_function(
        &mut self,
        s: &mut EmitState<'_>,
        id: NodeId,
        node: &Function,
        flags: EmitFlags,
    ) -> Result<()> {
        if node.parameters.is_empty() && s.written_returns(id).is_none() {
            s.keyword(Keyword::Function);
            self.emit_templates(s, id);
        } else {
            self.single_line(s, |this, s| {
                let parameters = s.emitted_parameters(s.ast().list(node.parameters));

                s.keyword(Keyword::Function);
                this.emit_templates(s, id);
                s.punct(Punct::LParen);
                this.emit_with_separators(s, &parameters, ", ")?;
                s.punct(Punct::RParen);
                this.emit_return_type(s, id);

                Ok(())
            })?;
        }

        if flags.header_only {
            return Ok(());
        }

        s.eol();
        self.with_indent(s, |this, s| this.emit_function_body(s, node))?;
        s.push_fmt(format_args!(
            "{}{}",
            self.indentation(),
            Keyword::EndFunction
        ));

        Ok(())
    }

    fn begin_line(&mut self, s: &mut EmitState<'_>) {
        self.push_indent(s);
    }

    fn emit_map(&mut self, s: &mut EmitState<'_>, id: NodeId, node: &Map) -> Result<()> {
        self.emit_collection(
            s,
            id,
            s.ast().list(node.fields),
            (Punct::LBrace.as_str(), Punct::RBrace.as_str()),
            false,
        )
    }

    fn emit_map_entry(
        &mut self,
        s: &mut EmitState<'_>,
        node: &MapEntry,
        flags: EmitFlags,
    ) -> Result<()> {
        if node.shorthand && s.options().dev_mode {
            return self.emit(s, node.value, flags);
        }

        self.emit_map_key(s, node)?;
        s.push(MAP_KEY_SEPARATOR);
        s.push(" ");

        self.emit(s, node.value, flags)
    }

    fn emit_list(&mut self, s: &mut EmitState<'_>, id: NodeId, node: &List) -> Result<()> {
        self.emit_collection(
            s,
            id,
            s.ast().list(node.fields),
            (Punct::LBracket.as_str(), Punct::RBracket.as_str()),
            true,
        )
    }

    fn emit_list_item(
        &mut self,
        s: &mut EmitState<'_>,
        node: &ListItem,
        flags: EmitFlags,
    ) -> Result<()> {
        self.emit(s, node.value, flags)
    }

    fn emit_literal(&mut self, s: &mut EmitState<'_>, id: NodeId, node: &Literal) -> Result<()> {
        if s.options().dev_mode
            && matches!(node.value, LiteralValue::Number(_))
            && !NumericLiteral::is_plain(s.text(id))
        {
            let sign = if node.negated { "-" } else { "" };
            let text = format!("{sign}{}", s.text(id));

            s.push(&text);

            return Ok(());
        }

        s.literal(
            id,
            node,
            s.text(id),
            LiteralStyle::Value,
            self.runtime_name(multiline_string_helper(s.options())),
        );

        Ok(())
    }

    fn emit_while(&mut self, s: &mut EmitState<'_>, node: &While) -> Result<()> {
        s.keyword(Keyword::While);
        s.push(" ");
        self.single_line(s, |this, s| {
            this.emit(s, node.condition, EmitFlags::default())
        })?;

        self.emit_block(s, s.ast().list(node.body), Keyword::EndWhile)
    }

    fn emit_call_statement(
        &mut self,
        s: &mut EmitState<'_>,
        node: &CallStatement,
        _flags: EmitFlags,
    ) -> Result<()> {
        let command = EmitFlags {
            is_command: true,
            ..EmitFlags::default()
        };

        self.emit(s, node.expression, command)
    }

    fn emit_call(
        &mut self,
        s: &mut EmitState<'_>,
        id: NodeId,
        node: &Call,
        flags: EmitFlags,
    ) -> Result<()> {
        self.emit(s, node.base, EmitFlags::default())?;
        if node.arguments.is_empty() {
            return Ok(());
        }

        let mut deferred_functions: Vec<NodeId> = s
            .ast()
            .list(node.arguments)
            .iter()
            .filter_map(|&argument| s.deferred_function(argument, id))
            .collect();

        if node.arguments.len() > MAX_INLINE_ARGUMENTS
            && !self.single_line
            && deferred_functions.is_empty()
        {
            self.with_indent(s, |this, s| {
                s.punct(Punct::LParen);
                s.eol();

                let last = node.arguments.len() - 1;

                for (index, &argument) in s.ast().list(node.arguments).iter().enumerate() {
                    this.push_indent(s);
                    this.emit(s, argument, EmitFlags::default())?;
                    if index != last {
                        s.punct(Punct::Comma);
                        s.eol();
                    }
                }

                Ok(())
            })?;
            s.punct(Punct::RParen);

            return Ok(());
        }

        let command = flags.is_command && !self.options.retain_parentheses;

        if command {
            s.push(" ");
        } else {
            s.punct(Punct::LParen);
        }

        for (index, &argument) in s.ast().list(node.arguments).iter().enumerate() {
            if index > 0 {
                s.punct(Punct::Comma);
                s.push(" ");
            }

            let argument_flags = EmitFlags {
                header_only: deferred_functions.contains(&s.unwrap_value(argument)),
                ..EmitFlags::default()
            };

            self.emit(s, argument, argument_flags)?;
        }

        if !command {
            s.punct(Punct::RParen);
        }

        deferred_functions.sort_by_key(|&function| s.end_line(function));
        for function in deferred_functions {
            self.emit_function_tail(s, function)?;
        }

        Ok(())
    }

    fn emit_slice(&mut self, s: &mut EmitState<'_>, node: &Slice) -> Result<()> {
        self.emit(s, node.base, EmitFlags::default())?;
        s.punct(Punct::LBracket);
        self.emit(s, node.left, EmitFlags::default())?;
        s.push(" ");
        s.push(SLICE_SEPARATOR);
        s.push(" ");
        self.emit(s, node.right, EmitFlags::default())?;
        s.punct(Punct::RBracket);

        Ok(())
    }

    fn emit_if(&mut self, s: &mut EmitState<'_>, id: NodeId, node: &If) -> Result<()> {
        let shortcut = s.is_shortcut(id, node.shortcut);

        for (index, &clause) in s.ast().list(node.clauses).iter().enumerate() {
            if shortcut && index > 0 {
                s.push(" ");
            }

            self.emit(s, clause, EmitFlags::default())?;
        }

        if !shortcut {
            s.push_fmt(format_args!("{}{}", self.indentation(), Keyword::EndIf));
        }

        Ok(())
    }

    fn emit_if_clause(&mut self, s: &mut EmitState<'_>, id: NodeId, node: &IfClause) -> Result<()> {
        let keyword = match node.kind {
            IfClauseKind::If => Keyword::If,
            IfClauseKind::ElseIf => Keyword::ElseIf,
        };

        if s.is_shortcut(id, node.shortcut) {
            s.keyword(keyword);
            s.push(" ");
            self.emit_without_parens(s, node.condition)?;
            s.push(" ");
            s.keyword(Keyword::Then);
            s.push(" ");

            return self.emit_first(s, s.ast().list(node.body));
        }

        if node.kind == IfClauseKind::ElseIf {
            self.push_indent(s);
        }

        s.keyword(keyword);
        s.push(" ");
        self.emit_without_parens(s, node.condition)?;
        s.push(" ");
        s.keyword(Keyword::Then);
        Self::emit_trailing_comments(s, id, None);
        s.eol();

        self.with_indent(s, |this, s| this.emit_body(s, s.ast().list(node.body)))
    }

    fn emit_else_clause(
        &mut self,
        s: &mut EmitState<'_>,
        id: NodeId,
        node: &ElseClause,
    ) -> Result<()> {
        if s.is_shortcut(id, node.shortcut) {
            s.keyword(Keyword::Else);
            s.push(" ");

            return self.emit_first(s, s.ast().list(node.body));
        }

        s.push_fmt(format_args!("{}{}", self.indentation(), Keyword::Else));
        Self::emit_trailing_comments(s, id, None);
        s.eol();

        self.with_indent(s, |this, s| this.emit_body(s, s.ast().list(node.body)))
    }

    fn emit_for(&mut self, s: &mut EmitState<'_>, node: &For) -> Result<()> {
        s.keyword(Keyword::For);
        s.push(" ");

        if s.options().dev_mode
            && let Some(header) = node.header
        {
            let names = s.ast().slice(header.span).to_owned();

            s.push(&names);
            s.push(" ");
            s.keyword(Keyword::In);
            s.push(" ");
            self.emit_without_parens(s, node.sequence)?;

            return self.emit_block(
                s,
                &s.ast().list(node.body)[header.prelude..],
                Keyword::EndFor,
            );
        }

        self.emit_without_parens(s, node.variable)?;
        s.push(" ");
        s.keyword(Keyword::In);
        s.push(" ");
        self.emit_without_parens(s, node.sequence)?;

        self.emit_block(s, s.ast().list(node.body), Keyword::EndFor)
    }

    fn emit_isa(&mut self, s: &mut EmitState<'_>, node: &Isa, flags: EmitFlags) -> Result<()> {
        let inner = EmitFlags {
            logical_continuation: flags.logical_continuation,
            ..EmitFlags::default()
        };

        self.emit(s, node.left, inner)?;
        s.push(" ");
        s.keyword(Keyword::Isa);
        s.push(" ");

        self.emit(s, node.right, inner)
    }

    fn emit_logical(
        &mut self,
        s: &mut EmitState<'_>,
        node: &Logical,
        flags: EmitFlags,
    ) -> Result<()> {
        let operator = node.operator.as_str();

        if s.count_operations(node.right) <= MAX_INLINE_OPERATIONS {
            let inner = EmitFlags {
                logical_continuation: flags.logical_continuation,
                ..EmitFlags::default()
            };

            self.emit(s, node.left, inner)?;
            s.push_fmt(format_args!(" {operator} "));

            return self.emit(s, node.right, inner);
        }

        let inner = EmitFlags {
            logical_continuation: true,
            ..EmitFlags::default()
        };
        let indent = usize::from(!flags.logical_continuation);

        self.indent += indent;

        let result = (|| {
            self.emit(s, node.left, inner)?;
            s.push_fmt(format_args!(" {operator} "));
            s.eol();
            self.push_indent(s);

            self.emit(s, node.right, inner)
        })();

        self.indent -= indent;

        result
    }

    fn emit_binary(&mut self, s: &mut EmitState<'_>, node: &Binary) -> Result<()> {
        self.emit(s, node.left, EmitFlags::default())?;
        s.push_fmt(format_args!(" {} ", node.operator.as_str()));

        self.emit(s, node.right, EmitFlags::default())
    }

    fn emit_comparison_group(
        &mut self,
        s: &mut EmitState<'_>,
        node: &ComparisonGroup,
    ) -> Result<()> {
        let Some((&first, remainder)) = s.ast().list(node.expressions).split_first() else {
            return Ok(());
        };

        self.emit(s, first, EmitFlags::default())?;
        for (operator, &expression) in node.operators.iter().zip(remainder) {
            s.push_fmt(format_args!(" {} ", operator.as_str()));
            self.emit(s, expression, EmitFlags::default())?;
        }

        Ok(())
    }

    fn emit_inject(&mut self, s: &mut EmitState<'_>, id: NodeId, node: &Inject) -> Result<()> {
        let content = s
            .module()
            .filter(|_| !s.options().dev_mode)
            .map(|module| module.injection(id));

        match content {
            None => {
                let text = s.read(node.path);

                s.keyword(Keyword::Inject);
                s.push(" ");
                s.write_quoted_text(&text);
                s.push(STATEMENT_SEPARATOR);
            }
            Some(Some(content)) => s.write_quoted_text(&content.replace(QUOTE, ESCAPED_QUOTE)),
            Some(None) => s.push(NULL),
        }

        Ok(())
    }

    fn emit_include(&mut self, s: &mut EmitState<'_>, id: NodeId, node: &Include) -> Result<()> {
        if s.options().dev_mode {
            let text = s.read(node.path);

            s.keyword(Keyword::Include);
            s.push(" ");
            if node.type_only {
                s.push(TYPE);
                s.push(" ");
            }

            s.write_quoted_text(&text);
            s.push(STATEMENT_SEPARATOR);

            return Ok(());
        }

        if node.type_only {
            return Ok(());
        }

        let Some(dependency) = s.dependency_module(id) else {
            {
                let text = s.read(node.path);

                s.keyword(Keyword::Include);
                s.push(" ");
                s.write_quoted_text(&text);
                s.push(STATEMENT_SEPARATOR);
            };

            return Ok(());
        };

        s.with_module(dependency, |s| {
            let root = s.ast().root();

            self.emit(s, root, EmitFlags::default())
        })
    }
}
