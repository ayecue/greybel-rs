use greybel_core::ast::NodeList;
use greybel_core::syntax::greybel::{
    Assert, AssertedType, Bitwise, BitwiseOp, Class, ClassMember, Enum, Envar, File, Import,
    Include, Inject, Switch, SwitchCase, Template, TypeAssertion,
};
use greybel_core::syntax::greyscript::ImportCode;
use greybel_core::syntax::{
    ArrowBody, Assignment, Binary, Call, CallStatement, ComparisonGroup, ElseClause, For, Function,
    Identifier, If, IfClause, Index, Isa, List, ListItem, Literal, Logical, Map, MapEntry, Member,
    Parenthesis, Return, Slice, Unary, While,
};
use greybel_core::{
    BinaryOp, ColumnSettings, Dialect, IfClauseKind, LiteralValue, NodeId, NodeKind, UnaryOp,
};
use greybel_symbols::contextual_keywords::{
    AS, ASSERT, CASE, CLASS, EXPORT, EXTENDS, FROM, IMPLEMENTS, IS, PRIVATE, STATIC, SWITCH, TYPE,
};
use greybel_symbols::delimiters::{
    ESCAPED_QUOTE, LINE_COMMENT, MAP_KEY_SEPARATOR, QUOTE, SLICE_SEPARATOR, STATEMENT_SEPARATOR,
};
use greybel_symbols::literals::NULL;
use greybel_symbols::names::{BIT_AND, BIT_OR, BITWISE, EXIT};
use greybel_symbols::{Keyword, Punct};

use super::nodes::{deferred_function, dependency_module, emitted_parameters, expression_default};
use super::{EmitFlags, EmitOutput, EmitState, LiteralStyle};
use crate::bundle::{MULTILINE_STRING, REQUIRE};
use crate::error::{Result, TranspileError};

/// How a template's line break is written in a build.
const LINE_BREAK_CHARACTER: &str = "char(10)";

pub trait Emitter: Sized {
    const ARGUMENT_SEPARATOR: &'static str = Punct::Comma.as_str();
    const INFIX_PADDING: &'static str = "";
    const RETAINS_COMMENTS: bool = false;

    fn finish(&self, output: EmitOutput) -> String {
        output.into_code()
    }

    fn preamble(&self, _s: &EmitState<'_>) -> Vec<String> {
        Vec::new()
    }

    /// Output name for a build-owned global (`RUNTIME_NAMES` or `MULTILINE_STRING`), assigned to its identifier.
    fn runtime_name<'a>(&'a self, name: &'a str) -> &'a str {
        name
    }

    fn emit(&mut self, s: &mut EmitState<'_>, id: NodeId, flags: EmitFlags) -> Result<()> {
        if !matches!(s.kind(id), NodeKind::Chunk(_) | NodeKind::Include(_)) {
            s.mark(id);
        }

        if !s.options().dev_mode && self.emit_expansions(s, id, flags)? {
            return Ok(());
        }

        if Self::is_value_expression(s, id) {
            return self.emit_value_expression(s, id);
        }

        if let Some(member) = s.class_member(id) {
            return self.emit_class_member(s, member);
        }

        match s.kind(id) {
            NodeKind::Chunk(chunk) => self.emit_body(s, s.ast().list(chunk.body)),
            NodeKind::Function(node) if node.arrow.is_some() && s.options().dev_mode => {
                self.emit_arrow(s, node, flags)
            }
            NodeKind::Function(node) => self.emit_function(s, id, node, flags),
            NodeKind::Assignment(node) => self.emit_assignment(s, id, node, flags),
            NodeKind::CallStatement(node) => self.emit_call_statement(s, node, flags),
            NodeKind::Return(_) if s.is_bare_return(id) => {
                s.keyword(Keyword::Return);

                Ok(())
            }
            NodeKind::Return(node) => self.emit_return(s, node),
            NodeKind::If(node) => self.emit_if(s, id, node),
            NodeKind::IfClause(node) => self.emit_if_clause(s, id, node),
            NodeKind::ElseClause(node) => self.emit_else_clause(s, id, node),
            NodeKind::While(node) => self.emit_while(s, node),
            NodeKind::For(node) => self.emit_for(s, node),
            NodeKind::Break(_) => self.emit_keyword(s, Keyword::Break),
            NodeKind::Continue(_) => self.emit_keyword(s, Keyword::Continue),
            NodeKind::Identifier(node) => self.emit_name(s, id, node, flags),
            NodeKind::Member(node) => self.emit_member_or_constant(s, id, node),
            NodeKind::Index(node) => self.emit_index(s, node),
            NodeKind::Slice(node) => self.emit_slice(s, node),
            NodeKind::Call(node) => self.emit_call(s, id, node, flags),
            NodeKind::Literal(node) => self.emit_literal(s, id, node),
            NodeKind::Unary(node) => self.emit_unary(s, node),
            NodeKind::Binary(node) => self.emit_binary(s, node),
            NodeKind::Logical(node) => self.emit_logical(s, node, flags),
            NodeKind::Isa(node) => self.emit_isa(s, node, flags),
            NodeKind::ComparisonGroup(node) => self.emit_comparison_group(s, node),
            NodeKind::Map(node) if !s.options().dev_mode && has_spread(s, node.fields) => {
                self.emit_spread_collection(s, node.fields, (Punct::LBrace, Punct::RBrace))
            }
            NodeKind::List(node) if !s.options().dev_mode && has_spread(s, node.fields) => {
                self.emit_spread_collection(s, node.fields, (Punct::LBracket, Punct::RBracket))
            }
            NodeKind::Spread(node) => {
                s.punct(Punct::Ellipsis);

                self.emit(s, node.argument, EmitFlags::default())
            }
            NodeKind::Map(node) => self.emit_map(s, id, node),
            NodeKind::MapEntry(node) => self.emit_map_entry(s, node, flags),
            NodeKind::List(node) => self.emit_list(s, id, node),
            NodeKind::ListItem(node) => self.emit_list_item(s, node, flags),
            NodeKind::Parenthesis(node) => self.emit_parenthesis(s, node, flags),
            NodeKind::Template(node) => self.emit_template(s, id, node),
            NodeKind::Enum(node) => self.emit_enum(s, id, node),
            NodeKind::Assert(node) => self.emit_assert(s, id, node),
            NodeKind::Export(_)
            | NodeKind::Switch(_)
            | NodeKind::DestructuringAssignment(_)
            | NodeKind::Class(_)
            | NodeKind::TypeStatement(_) => self.emit_statement_sugar(s, id),
            NodeKind::Ternary(_) | NodeKind::Coalesce(_) => self.emit_value_expression(s, id),
            NodeKind::TypeAssertion(node) => self.emit_type_assertion(s, node, flags),
            NodeKind::Empty(_)
            | NodeKind::Unknown(_)
            | NodeKind::Invalid(_)
            | NodeKind::Noop(_) => Ok(()),
            NodeKind::Import(node) => self.emit_import(s, id, node),
            NodeKind::Include(node) => self.emit_include(s, id, node),
            NodeKind::Envar(node) => self.emit_envar(s, node),
            NodeKind::Inject(node) => self.emit_inject(s, id, node),
            NodeKind::Debugger(_) => {
                if !s.options().dev_mode && !s.options().retain_debugger {
                    s.push(LINE_COMMENT);
                }

                self.emit_keyword(s, Keyword::Debugger)
            }
            NodeKind::Line(_) => {
                if s.options().dev_mode {
                    s.keyword(Keyword::Line);
                } else {
                    let line = s.start_line(id).to_string();

                    s.push(&line);
                }

                Ok(())
            }
            NodeKind::File(node) => self.emit_file(s, node),
            NodeKind::Bitwise(node) => self.emit_bitwise(s, node),
            NodeKind::ImportCode(node) => {
                self.emit_import_code(s, id, node);

                Ok(())
            }
        }
    }

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

    fn emit_import_code(&mut self, s: &mut EmitState<'_>, id: NodeId, node: &ImportCode) {
        if s.options().dev_mode {
            s.push_import_code_source(node, !Self::RETAINS_COMMENTS);

            return;
        }

        if !node.emit {
            return;
        }

        if !node.eval || s.graph().is_none() {
            {
                let text = s.read(node.directory);

                s.keyword(Keyword::ImportCode);
                s.punct(Punct::LParen);
                s.write_quoted_text(&text);
                s.punct(Punct::RParen);
            };

            return;
        }

        for (index, path) in s.bundled_imports(id).iter().enumerate() {
            if index > 0 {
                s.eol();
            }

            s.keyword(Keyword::ImportCode);
            s.punct(Punct::LParen);
            s.write_quoted_text(path);
            s.punct(Punct::RParen);
        }
    }

    fn emit_keyword(&mut self, s: &mut EmitState<'_>, keyword: Keyword) -> Result<()> {
        s.keyword(keyword);

        Ok(())
    }

    fn emit_identifier(
        &mut self,
        s: &mut EmitState<'_>,
        node: &Identifier,
        _flags: EmitFlags,
    ) -> Result<()> {
        s.push(s.resolve(node.name));

        Ok(())
    }

    fn emit_body(&mut self, s: &mut EmitState<'_>, body: &[NodeId]) -> Result<()> {
        for &item in body {
            if s.is_noop(item) || s.is_omitted(item) {
                continue;
            }

            self.emit(s, item, EmitFlags::default())?;
            self.end_statement(s);
        }

        Ok(())
    }

    fn end_statement(&mut self, s: &mut EmitState<'_>) {
        s.eol();
    }

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

    /// Emits the body of a function, starting with the defaults that Grey Hack can't take as parameter defaults.
    fn emit_function_body(&mut self, s: &mut EmitState<'_>, node: &Function) -> Result<()> {
        self.emit_expression_defaults(s, node)?;

        self.emit_body(s, s.ast().list(node.body))
    }

    /// Emits `if @name == null then name = default` for each parameter whose default isn't a literal.
    fn emit_expression_defaults(&mut self, s: &mut EmitState<'_>, node: &Function) -> Result<()> {
        for &parameter in s.ast().list(node.parameters) {
            let Some(assignment) = expression_default(s, parameter) else {
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

    /// Starts a line written by the emitter itself rather than for a statement of the source.
    fn begin_line(&mut self, _s: &mut EmitState<'_>) {}

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

    fn emit_parameters(&mut self, s: &mut EmitState<'_>, parameters: &[NodeId]) -> Result<()> {
        let parameters = emitted_parameters(s, parameters);

        self.emit_with_separators(s, &parameters, Punct::Comma.as_str())
    }

    fn emit_assignment(
        &mut self,
        s: &mut EmitState<'_>,
        _id: NodeId,
        node: &Assignment,
        _flags: EmitFlags,
    ) -> Result<()> {
        self.emit(s, node.variable, EmitFlags::default())?;
        s.punct(Punct::Assign);

        self.emit(s, node.init, EmitFlags::default())
    }

    fn emit_call_statement(
        &mut self,
        s: &mut EmitState<'_>,
        node: &CallStatement,
        _flags: EmitFlags,
    ) -> Result<()> {
        self.emit(s, node.expression, EmitFlags::default())
    }

    fn emit_return(&mut self, s: &mut EmitState<'_>, node: &Return) -> Result<()> {
        s.keyword(Keyword::Return);
        s.push(" ");

        match node.argument {
            Some(argument) => self.emit(s, argument, EmitFlags::default()),
            None => Ok(()),
        }
    }

    fn emit_if(&mut self, s: &mut EmitState<'_>, id: NodeId, node: &If) -> Result<()> {
        let shortcut = s.is_shortcut(id, node.shortcut);

        if shortcut {
            for (index, &clause) in s.ast().list(node.clauses).iter().enumerate() {
                if index > 0 {
                    s.push(" ");
                }

                self.emit(s, clause, EmitFlags::default())?;
            }

            return Ok(());
        }

        for &clause in s.ast().list(node.clauses) {
            self.emit(s, clause, EmitFlags::default())?;
        }

        s.keyword(Keyword::EndIf);

        Ok(())
    }

    fn emit_if_clause(&mut self, s: &mut EmitState<'_>, id: NodeId, node: &IfClause) -> Result<()> {
        s.keyword(match node.kind {
            IfClauseKind::If => Keyword::If,
            IfClauseKind::ElseIf => Keyword::ElseIf,
        });
        s.push(" ");
        self.emit(s, node.condition, EmitFlags::default())?;
        if s.is_shortcut(id, node.shortcut) {
            s.push(" ");
            s.keyword(Keyword::Then);
            s.push(" ");

            return self.emit_first(s, s.ast().list(node.body));
        }

        s.push(" ");
        s.keyword(Keyword::Then);
        s.eol();

        self.emit_body(s, s.ast().list(node.body))
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

        s.keyword(Keyword::Else);
        s.eol();

        self.emit_body(s, s.ast().list(node.body))
    }

    fn emit_first(&mut self, s: &mut EmitState<'_>, body: &[NodeId]) -> Result<()> {
        match body.first() {
            Some(&item) => self.emit(s, item, EmitFlags::default()),
            None => Ok(()),
        }
    }

    fn emit_while(&mut self, s: &mut EmitState<'_>, node: &While) -> Result<()> {
        s.keyword(Keyword::While);
        s.push(" ");
        self.emit(s, node.condition, EmitFlags::default())?;
        s.eol();
        self.emit_body(s, s.ast().list(node.body))?;
        s.keyword(Keyword::EndWhile);

        Ok(())
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
            self.emit(s, node.sequence, EmitFlags::default())?;
            s.eol();
            self.emit_body(s, &s.ast().list(node.body)[header.prelude..])?;
            s.keyword(Keyword::EndFor);

            return Ok(());
        }

        let variable = EmitFlags {
            is_for_loop_variable: true,
            ..EmitFlags::default()
        };

        self.emit(s, node.variable, variable)?;
        s.push(" ");
        s.keyword(Keyword::In);
        s.push(" ");
        self.emit(s, node.sequence, EmitFlags::default())?;
        s.eol();
        self.emit_body(s, s.ast().list(node.body))?;
        s.keyword(Keyword::EndFor);

        Ok(())
    }

    /// A member read, or the value of the `const enum` entry it reads.
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

    fn emit_member(&mut self, s: &mut EmitState<'_>, node: &Member) -> Result<()> {
        self.emit(s, node.base, EmitFlags::default())?;
        s.punct(Punct::Dot);

        let identifier = EmitFlags {
            is_member: true,
            ..EmitFlags::default()
        };

        self.emit(s, node.identifier, identifier)
    }

    fn emit_index(&mut self, s: &mut EmitState<'_>, node: &Index) -> Result<()> {
        self.emit(s, node.base, EmitFlags::default())?;
        s.punct(Punct::LBracket);
        self.emit(s, node.index, EmitFlags::default())?;
        s.punct(Punct::RBracket);

        Ok(())
    }

    fn emit_slice(&mut self, s: &mut EmitState<'_>, node: &Slice) -> Result<()> {
        self.emit(s, node.base, EmitFlags::default())?;
        s.punct(Punct::LBracket);
        self.emit(s, node.left, EmitFlags::default())?;
        s.push(SLICE_SEPARATOR);
        self.emit(s, node.right, EmitFlags::default())?;
        s.punct(Punct::RBracket);

        Ok(())
    }

    fn emit_call(
        &mut self,
        s: &mut EmitState<'_>,
        id: NodeId,
        node: &Call,
        _flags: EmitFlags,
    ) -> Result<()> {
        self.emit(s, node.base, EmitFlags::default())?;

        self.emit_call_arguments(s, id, node)
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

    fn emit_literal(&mut self, s: &mut EmitState<'_>, id: NodeId, node: &Literal) -> Result<()> {
        s.literal(
            node,
            s.text(id),
            LiteralStyle::Output,
            self.runtime_name(MULTILINE_STRING),
        );

        Ok(())
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

    fn emit_binary(&mut self, s: &mut EmitState<'_>, node: &Binary) -> Result<()> {
        self.emit(s, node.left, EmitFlags::default())?;
        s.push(node.operator.as_str());

        self.emit(s, node.right, EmitFlags::default())
    }

    fn emit_logical(
        &mut self,
        s: &mut EmitState<'_>,
        node: &Logical,
        _flags: EmitFlags,
    ) -> Result<()> {
        self.emit(s, node.left, EmitFlags::default())?;
        s.push(" ");
        s.push(node.operator.as_str());
        s.push(" ");

        self.emit(s, node.right, EmitFlags::default())
    }

    fn emit_isa(&mut self, s: &mut EmitState<'_>, node: &Isa, _flags: EmitFlags) -> Result<()> {
        self.emit(s, node.left, EmitFlags::default())?;
        s.push(" ");
        s.keyword(Keyword::Isa);
        s.push(" ");

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
            s.push(operator.as_str());
            self.emit(s, expression, EmitFlags::default())?;
        }

        Ok(())
    }

    fn emit_map(&mut self, s: &mut EmitState<'_>, id: NodeId, node: &Map) -> Result<()> {
        s.punct(Punct::LBrace);

        let deferred_functions =
            self.emit_items(s, id, s.ast().list(node.fields), Punct::Comma.as_str())?;

        s.punct(Punct::RBrace);

        self.emit_deferred_functions(s, &deferred_functions)
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

        self.emit(s, node.value, flags)
    }

    /// The key of a map entry; `{name}` gets its key as a string.
    fn emit_map_key(&mut self, s: &mut EmitState<'_>, node: &MapEntry) -> Result<()> {
        if node.shorthand {
            let name = s.text(node.key).to_owned();

            s.write_quoted_text(&name);

            return Ok(());
        }

        self.emit(s, node.key, EmitFlags::default())
    }

    /// A list or map literal with spreads as additions: `[...a, 1]` as `([] + a + [1])`. Parentheses group the
    /// additions together where the literal was.
    fn emit_spread_collection(
        &mut self,
        s: &mut EmitState<'_>,
        fields: NodeList,
        (open, close): (Punct, Punct),
    ) -> Result<()> {
        let fields = s.ast().list(fields).to_vec();
        let plus = format!("{0}+{0}", Self::INFIX_PADDING);
        let mut index = 0;

        s.punct(Punct::LParen);

        if matches!(s.kind(fields[0]), NodeKind::Spread(_)) {
            s.punct(open);
            s.punct(close);
        }

        while index < fields.len() {
            if index > 0 || matches!(s.kind(fields[0]), NodeKind::Spread(_)) {
                s.push(&plus);
            }

            if let NodeKind::Spread(spread) = s.kind(fields[index]) {
                let argument = spread.argument;

                if is_operand(s, argument) {
                    self.emit(s, argument, EmitFlags::default())?;
                } else {
                    s.punct(Punct::LParen);
                    self.emit(s, argument, EmitFlags::default())?;
                    s.punct(Punct::RParen);
                }

                index += 1;

                continue;
            }

            let end = fields[index..]
                .iter()
                .position(|&field| matches!(s.kind(field), NodeKind::Spread(_)))
                .map_or(fields.len(), |offset| index + offset);

            s.punct(open);

            for (position, &field) in fields[index..end].iter().enumerate() {
                if position > 0 {
                    s.push(Self::ARGUMENT_SEPARATOR);
                }

                self.emit(s, field, EmitFlags::default())?;
            }

            s.punct(close);
            index = end;
        }

        s.punct(Punct::RParen);

        Ok(())
    }

    fn emit_list(&mut self, s: &mut EmitState<'_>, id: NodeId, node: &List) -> Result<()> {
        s.punct(Punct::LBracket);

        let deferred_functions =
            self.emit_items(s, id, s.ast().list(node.fields), Punct::Comma.as_str())?;

        s.punct(Punct::RBracket);

        self.emit_deferred_functions(s, &deferred_functions)
    }

    fn emit_list_item(
        &mut self,
        s: &mut EmitState<'_>,
        node: &ListItem,
        flags: EmitFlags,
    ) -> Result<()> {
        self.emit(s, node.value, flags)
    }

    /// `#"a{x}b"` as `"a" + (x) + "b"`; empty text pieces are left out, and `""` starts it when the first piece
    /// is empty, so the result is always a string. A line break in a piece is written as `char(10)`. Dev mode
    /// retains the template as written.
    fn emit_template(&mut self, s: &mut EmitState<'_>, id: NodeId, node: &Template) -> Result<()> {
        if s.options().dev_mode {
            let text = s.text(id).to_owned();

            s.push(&text);

            return Ok(());
        }

        let source = s.ast().source();
        let pieces: Vec<String> = node
            .texts
            .iter()
            .map(|&span| Template::text(source, span).into_owned())
            .collect();
        let expressions = s.ast().list(node.expressions).to_vec();
        let plus = format!("{0}+{0}", Self::INFIX_PADDING);
        let mut first = true;

        for (index, piece) in pieces.iter().enumerate() {
            if !piece.is_empty() || (index == 0 && !expressions.is_empty()) || pieces.len() == 1 {
                if !first {
                    s.push(&plus);
                }

                Self::push_template_text(s, piece, &plus);
                first = false;
            }

            if let Some(&expression) = expressions.get(index) {
                if !first {
                    s.push(&plus);
                }

                s.punct(Punct::LParen);
                self.emit(s, expression, EmitFlags::default())?;
                s.punct(Punct::RParen);
                first = false;
            }
        }

        Ok(())
    }

    /// `assert condition, message` as `if not (condition) then exit(message)`; nothing when the build variable
    /// [`Assert::REMOVAL_VARIABLE`] is set.
    fn emit_assert(&mut self, s: &mut EmitState<'_>, id: NodeId, node: &Assert) -> Result<()> {
        if s.options().dev_mode {
            s.push(ASSERT);
            s.push(" ");
            self.emit(s, node.condition, EmitFlags::default())?;

            if let Some(message) = node.message {
                s.push(Self::ARGUMENT_SEPARATOR);
                self.emit(s, message, EmitFlags::default())?;
            }

            return Ok(());
        }

        if Assert::is_removed(&s.options().environment) {
            return Ok(());
        }

        s.keyword(Keyword::If);
        s.push(" ");
        s.keyword(Keyword::Not);
        s.push(" ");
        s.punct(Punct::LParen);
        self.emit(s, node.condition, EmitFlags::default())?;
        s.punct(Punct::RParen);
        s.push(" ");
        s.keyword(Keyword::Then);
        s.push(" ");
        s.push(EXIT);
        s.punct(Punct::LParen);

        match node.message {
            Some(message) => self.emit(s, message, EmitFlags::default())?,
            None => {
                let message = Assert::default_message(&node.filename, s.start_line(id));

                s.write_quoted_text(&message);
            }
        }

        s.punct(Punct::RParen);

        Ok(())
    }

    /// Statements one per line at the current indentation, for syntax sugar that stands for several statements.
    fn emit_statements(&mut self, s: &mut EmitState<'_>, statements: &[NodeId]) -> Result<()> {
        for (index, &statement) in statements.iter().enumerate() {
            if index > 0 {
                self.separate_statements(s);
            }

            self.emit(s, statement, EmitFlags::default())?;
        }

        Ok(())
    }

    /// `export`, `switch`, destructuring assignments and classes: as written in dev mode, else the statements they stand for.
    fn emit_statement_sugar(&mut self, s: &mut EmitState<'_>, id: NodeId) -> Result<()> {
        let statements = match s.kind(id) {
            NodeKind::Export(node) if s.options().dev_mode => {
                s.push(EXPORT);
                s.push(" ");

                return self.emit(s, node.assignment, EmitFlags::default());
            }
            NodeKind::Switch(node) if s.options().dev_mode => return self.emit_switch(s, node),
            NodeKind::Class(node) if s.options().dev_mode => return self.emit_class(s, node),
            NodeKind::DestructuringAssignment(_) | NodeKind::TypeStatement(_)
                if s.options().dev_mode =>
            {
                self.emit_block_source(s, id);

                return Ok(());
            }
            NodeKind::Export(node) => node.statements,
            NodeKind::Switch(node) => node.statements,
            NodeKind::DestructuringAssignment(node) => node.statements,
            NodeKind::Class(node) => node.statements,
            _ => return Ok(()),
        };
        let statements = s.ast().list(statements).to_vec();

        self.emit_statements(s, &statements)
    }

    /// The expression of `x!` and `x as T`; dev mode writes `!` and `as T` too.
    fn emit_type_assertion(
        &mut self,
        s: &mut EmitState<'_>,
        node: &TypeAssertion,
        flags: EmitFlags,
    ) -> Result<()> {
        self.emit(s, node.expression, flags)?;

        if s.options().dev_mode {
            match node.target {
                AssertedType::NonNull => s.push(Punct::Bang.as_str()),
                AssertedType::Type(span) => {
                    let ty = s.ast().slice(span).to_owned();

                    s.push_fmt(format_args!(" {AS} {ty}"));
                }
            }
        }

        Ok(())
    }

    /// An identifier, then the type written after it in dev mode.
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

    /// `: T` after a parameter or `let` name, as written, in dev mode.
    fn emit_written_type(&mut self, s: &mut EmitState<'_>, id: NodeId) {
        if let Some(ty) = s.written_type(id) {
            s.push_fmt(format_args!(": {ty}"));
        }
    }

    /// `<T, U>` after `function`, as written, in dev mode.
    fn emit_templates(&mut self, s: &mut EmitState<'_>, function: NodeId) {
        if let Some(names) = s.written_templates(function) {
            s.push_fmt(format_args!("<{names}>"));
        }
    }

    /// `: T` after the parameters, as written, in dev mode.
    fn emit_return_type(&mut self, s: &mut EmitState<'_>, function: NodeId) {
        if let Some(ty) = s.written_returns(function) {
            s.push_fmt(format_args!(": {ty}"));
        }
    }

    /// A class as written, in dev mode.
    fn emit_class(&mut self, s: &mut EmitState<'_>, node: &Class) -> Result<()> {
        s.enter_class(node);
        s.push(CLASS);
        s.push(" ");
        self.emit(s, node.name, EmitFlags::default())?;

        if let Some(base) = node.base {
            s.push_fmt(format_args!(" {EXTENDS} "));
            self.emit(s, base, EmitFlags::default())?;
        }

        for (index, &name) in node.implements.iter().enumerate() {
            if index == 0 {
                s.push_fmt(format_args!(" {IMPLEMENTS} "));
            } else {
                s.push(Self::ARGUMENT_SEPARATOR);
            }

            let name = s.ast().slice(name).to_owned();

            s.push(&name);
        }

        self.emit_block_body(s, s.ast().list(node.body), Keyword::EndClass)
    }

    /// A class member as written, in dev mode: `[private] name = value`, or `[private] name(parameters)` with its
    /// body (the written one for the constructor).
    fn emit_class_member(&mut self, s: &mut EmitState<'_>, member: ClassMember) -> Result<()> {
        let NodeKind::Assignment(assignment) = s.kind(member.assignment) else {
            return Ok(());
        };
        let NodeKind::Member(target) = s.kind(assignment.variable) else {
            return Ok(());
        };

        if member.private {
            s.push_fmt(format_args!("{PRIVATE} "));
        }

        if member.is_static {
            s.push_fmt(format_args!("{STATIC} "));
        }

        let name = EmitFlags {
            is_member: true,
            ..EmitFlags::default()
        };

        self.emit(s, target.identifier, name)?;

        if member.method
            && let NodeKind::Function(function) = s.kind(assignment.init)
        {
            let parameters = emitted_parameters(s, s.ast().list(function.parameters));

            s.punct(Punct::LParen);

            self.emit_with_separators(s, &parameters, Self::ARGUMENT_SEPARATOR)?;
            s.punct(Punct::RParen);
            self.emit_return_type(s, assignment.init);

            let body = s.written_body(assignment.init).unwrap_or(function.body);

            return self.emit_block_body(s, s.ast().list(body), Keyword::EndFunction);
        }

        s.push(Self::INFIX_PADDING);
        s.punct(Punct::Assign);
        s.push(Self::INFIX_PADDING);

        self.emit(s, assignment.init, EmitFlags::default())
    }

    /// The statements of a block on the lines after its header, then `end`.
    fn emit_block_body(
        &mut self,
        s: &mut EmitState<'_>,
        body: &[NodeId],
        end: Keyword,
    ) -> Result<()> {
        s.eol();
        self.emit_body(s, body)?;
        s.keyword(end);

        Ok(())
    }

    /// The statements builds write for value expressions (see `ValueExpansions`): those that replace `id`, the
    /// temporary read in its place, or those before it. Returns whether `id` is written by them.
    fn emit_expansions(
        &mut self,
        s: &mut EmitState<'_>,
        id: NodeId,
        flags: EmitFlags,
    ) -> Result<bool> {
        let expansions = &s.ast().expansions;

        if let Some(statements) = expansions.instead(id) {
            let statements = s.ast().list(statements).to_vec();

            self.emit_statements(s, &statements)?;

            return Ok(true);
        }

        if let Some(value) = expansions.value(id) {
            self.emit(s, value, flags)?;

            return Ok(true);
        }

        if let Some(statements) = expansions.before(id) {
            let statements = s.ast().list(statements).to_vec();

            self.emit_statements(s, &statements)?;
            self.separate_statements(s);
        }

        Ok(false)
    }

    /// Ends a statement of [`Self::emit_statements`] and starts the next one.
    fn separate_statements(&mut self, s: &mut EmitState<'_>) {
        s.eol();
    }

    /// Whether `id` is `a ? b : c`, `a ?? b` or a `?.` link.
    fn is_value_expression(s: &EmitState<'_>, id: NodeId) -> bool {
        match s.kind(id) {
            NodeKind::Ternary(_) | NodeKind::Coalesce(_) => true,
            NodeKind::Member(node) => node.optional,
            NodeKind::Index(node) => node.optional,
            NodeKind::Slice(node) => node.optional,
            NodeKind::Call(node) => node.optional,
            _ => false,
        }
    }

    /// A value expression as written, in dev mode. Builds only reach one that has no expansion.
    fn emit_value_expression(&mut self, s: &mut EmitState<'_>, id: NodeId) -> Result<()> {
        if !s.options().dev_mode {
            let position = s
                .ast()
                .lines()
                .position(s.ast().span(id).start, ColumnSettings::default());

            return Err(TranspileError::UnexpandedValue {
                file: s.source_label(),
                line: position.line,
                column: position.character + 1,
            });
        }

        let padding = Self::INFIX_PADDING;

        match s.kind(id) {
            NodeKind::Ternary(node) => {
                self.emit(s, node.condition, EmitFlags::default())?;
                s.push_fmt(format_args!("{padding}{}{padding}", Punct::Question));
                self.emit(s, node.if_true, EmitFlags::default())?;
                s.push_fmt(format_args!("{padding}{SLICE_SEPARATOR}{padding}"));

                self.emit(s, node.if_false, EmitFlags::default())
            }
            NodeKind::Coalesce(node) => {
                self.emit(s, node.left, EmitFlags::default())?;
                s.push_fmt(format_args!("{padding}{}{padding}", Punct::Coalesce));

                self.emit(s, node.right, EmitFlags::default())
            }
            NodeKind::Member(node) => {
                self.emit(s, node.base, EmitFlags::default())?;
                s.punct(Punct::OptionalDot);

                let identifier = EmitFlags {
                    is_member: true,
                    ..EmitFlags::default()
                };

                self.emit(s, node.identifier, identifier)
            }
            NodeKind::Index(node) => {
                self.emit(s, node.base, EmitFlags::default())?;
                s.punct(Punct::OptionalDot);
                s.punct(Punct::LBracket);
                self.emit(s, node.index, EmitFlags::default())?;
                s.punct(Punct::RBracket);

                Ok(())
            }
            NodeKind::Slice(node) => {
                self.emit(s, node.base, EmitFlags::default())?;
                s.punct(Punct::OptionalDot);
                s.punct(Punct::LBracket);
                self.emit(s, node.left, EmitFlags::default())?;
                s.push(SLICE_SEPARATOR);
                self.emit(s, node.right, EmitFlags::default())?;
                s.punct(Punct::RBracket);

                Ok(())
            }
            NodeKind::Call(node) => {
                self.emit(s, node.base, EmitFlags::default())?;
                s.punct(Punct::OptionalDot);
                s.punct(Punct::LParen);

                for (index, &argument) in s.ast().list(node.arguments).iter().enumerate() {
                    if index > 0 {
                        s.push(Self::ARGUMENT_SEPARATOR);
                    }

                    self.emit(s, argument, EmitFlags::default())?;
                }

                s.punct(Punct::RParen);

                Ok(())
            }
            _ => Ok(()),
        }
    }

    /// A `switch` as written, in dev mode.
    fn emit_switch(&mut self, s: &mut EmitState<'_>, node: &Switch) -> Result<()> {
        s.push(SWITCH);
        s.push(" ");
        self.emit(s, node.subject, EmitFlags::default())?;
        s.eol();

        for case in &node.cases {
            self.emit_switch_case(s, case)?;
            s.eol();
            self.emit_body(s, s.ast().list(case.body))?;
        }

        if let Some(fallback) = node.fallback {
            s.keyword(Keyword::Else);
            s.eol();
            self.emit_body(s, s.ast().list(fallback))?;
        }

        s.keyword(Keyword::EndSwitch);

        Ok(())
    }

    /// `case a, b` or `case is Type`.
    fn emit_switch_case(&mut self, s: &mut EmitState<'_>, case: &SwitchCase) -> Result<()> {
        s.push(CASE);
        s.push(" ");

        if case.type_case {
            s.push(IS);
            s.push(" ");
        }

        for (index, &value) in case.values.iter().enumerate() {
            if index > 0 {
                s.push(Self::ARGUMENT_SEPARATOR);
            }

            self.emit(s, value, EmitFlags::default())?;
        }

        Ok(())
    }

    /// A `(x) => ...` function as written, in dev mode.
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

    /// The lines after `(x) =>` of a block arrow function, up to `end function`.
    fn emit_arrow_body(&mut self, s: &mut EmitState<'_>, node: &Function) -> Result<()> {
        s.eol();
        self.emit_function_body(s, node)?;
        s.keyword(Keyword::EndFunction);

        Ok(())
    }

    /// Writes the source of a block statement (such as `enum`) as it is.
    fn emit_block_source(&mut self, s: &mut EmitState<'_>, id: NodeId) {
        let text = s.text(id).to_owned();

        s.push(&text);
    }

    /// `enum Color` as `Color = {"Red": 0, ...}`; dev mode retains the enum as written.
    fn emit_enum(&mut self, s: &mut EmitState<'_>, id: NodeId, node: &Enum) -> Result<()> {
        if s.options().dev_mode {
            self.emit_block_source(s, id);

            return Ok(());
        }

        let NodeKind::Assignment(assignment) = s.kind(node.assignment) else {
            return Ok(());
        };
        let (variable, init) = (assignment.variable, assignment.init);
        let NodeKind::Map(map) = s.kind(init) else {
            return Ok(());
        };
        let entries = s.ast().list(map.fields).to_vec();
        let padding = Self::INFIX_PADDING;

        self.emit(s, variable, EmitFlags::default())?;
        s.push_fmt(format_args!("{padding}={padding}"));
        s.punct(Punct::LBrace);

        for (index, &entry) in entries.iter().enumerate() {
            let NodeKind::MapEntry(entry) = s.kind(entry) else {
                continue;
            };
            let (key, value) = (entry.key, entry.value);

            if index > 0 {
                s.push_fmt(format_args!(",{padding}"));
            }

            let name = s.text(key).to_owned();

            s.write_quoted_text(&name);
            s.push_fmt(format_args!(":{padding}"));

            match s.kind(value) {
                NodeKind::Literal(Literal {
                    value: LiteralValue::Number(number),
                    negated,
                }) => {
                    let number = if *negated { -number } else { *number };

                    s.push_fmt(format_args!("{number}"));
                }
                _ => self.emit(s, value, EmitFlags::default())?,
            }
        }

        s.punct(Punct::RBrace);

        Ok(())
    }

    fn push_template_text(s: &mut EmitState<'_>, text: &str, plus: &str) {
        let source_text = text.replace(QUOTE, ESCAPED_QUOTE);

        for (index, line) in source_text.split('\n').enumerate() {
            if index > 0 {
                s.push(plus);
                s.push(LINE_BREAK_CHARACTER);
                s.push(plus);
            }

            s.write_quoted_text(line.strip_suffix('\r').unwrap_or(line));
        }
    }

    fn emit_parenthesis(
        &mut self,
        s: &mut EmitState<'_>,
        node: &Parenthesis,
        _flags: EmitFlags,
    ) -> Result<()> {
        s.punct(Punct::LParen);
        self.emit(s, node.expression, EmitFlags::default())?;
        s.punct(Punct::RParen);

        Ok(())
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

            match deferred_function(s, item, container) {
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

    fn emit_envar(&mut self, s: &mut EmitState<'_>, node: &Envar) -> Result<()> {
        if s.options().dev_mode {
            {
                let text = s.read(node.name);

                s.keyword(Keyword::Envar);
                s.push(" ");
                s.push(&text);
            };

            return Ok(());
        }

        match s.options().environment.get(s.read(node.name).as_ref()) {
            Some(value) if !value.is_empty() => {
                s.write_quoted_text(&value.replace(QUOTE, ESCAPED_QUOTE));
            }
            _ => s.push(NULL),
        }

        Ok(())
    }

    fn emit_inject(&mut self, s: &mut EmitState<'_>, id: NodeId, node: &Inject) -> Result<()> {
        if s.options().dev_mode {
            {
                let text = s.read(node.path);

                s.keyword(Keyword::Inject);
                s.push(" ");
                s.write_quoted_text(&text);
            };

            return Ok(());
        }

        let Some(module) = s.module() else {
            {
                let text = s.read(node.path);

                s.keyword(Keyword::Inject);
                s.push(" ");
                s.write_quoted_text(&text);
                s.push(STATEMENT_SEPARATOR);
            };

            return Ok(());
        };

        match module.injection(id) {
            Some(content) => s.write_quoted_text(&content.replace(QUOTE, ESCAPED_QUOTE)),
            None => s.push(NULL),
        }

        Ok(())
    }

    fn emit_import(&mut self, s: &mut EmitState<'_>, id: NodeId, node: &Import) -> Result<()> {
        let Some(dependency) = dependency_module(s, id) else {
            s.keyword(Keyword::Import);
            s.push(" ");

            match node.target {
                Some(target) => {
                    let names = s.ast().slice(target).to_owned();

                    s.push(&names);
                }
                None => self.emit(s, node.name, EmitFlags::default())?,
            }

            {
                let text = s.read(node.path);

                s.push(" ");
                s.push(FROM);
                s.push(" ");
                s.write_quoted_text(&text);
                s.push(STATEMENT_SEPARATOR);
            };

            return Ok(());
        };

        self.emit(s, node.name, EmitFlags::default())?;

        let call = self.require_assignment(s, s.namespace(dependency));

        s.push(&call);

        let bindings = s.ast().list(node.bindings).to_vec();

        if bindings.is_empty() {
            return Ok(());
        }

        s.eol();

        self.emit_statements(s, &bindings)
    }

    fn require_assignment(&self, _s: &EmitState<'_>, namespace: &str) -> String {
        format!(
            " {} {REQUIRE}{}{QUOTE}{namespace}{QUOTE}{}",
            Punct::Assign,
            Punct::LParen,
            Punct::RParen
        )
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

        let Some(dependency) = s.module().and_then(|module| module.dependency(id)) else {
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

    fn emit_file(&mut self, s: &mut EmitState<'_>, node: &File) -> Result<()> {
        if s.options().dev_mode {
            s.keyword(Keyword::Filename);

            return Ok(());
        }

        let name = node.filename.rsplit(['/', '\\']).next().unwrap_or_default();

        s.write_quoted_text(name);

        Ok(())
    }
}

fn has_spread(s: &EmitState<'_>, fields: NodeList) -> bool {
    s.ast()
        .list(fields)
        .iter()
        .any(|&field| matches!(s.kind(field), NodeKind::Spread(_)))
}

/// Whether `node` can be an operand of `+` without parentheses.
fn is_operand(s: &EmitState<'_>, node: NodeId) -> bool {
    matches!(
        s.kind(node),
        NodeKind::Identifier(_)
            | NodeKind::Member(_)
            | NodeKind::Index(_)
            | NodeKind::Call(_)
            | NodeKind::List(_)
            | NodeKind::Map(_)
            | NodeKind::Parenthesis(_)
            | NodeKind::Literal(_)
    )
}
