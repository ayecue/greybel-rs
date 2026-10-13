use greybel_core::syntax::greybel::{Bitwise, BitwiseOp, Envar, File, Import, Include, Inject};
use greybel_core::syntax::greyscript::ImportCode;
use greybel_core::syntax::{
    Assignment, Binary, Call, CallStatement, ComparisonGroup, ElseClause, For, Function,
    Identifier, If, IfClause, Index, Isa, List, ListItem, Literal, Logical, Map, MapEntry, Member,
    Parenthesis, Return, Slice, Unary, While,
};
use greybel_core::{Dialect, IfClauseKind, NodeId, NodeKind, UnaryOp};
use greybel_symbols::contextual_keywords::{FROM, TYPE};
use greybel_symbols::delimiters::{
    ESCAPED_QUOTE, LINE_COMMENT, MAP_KEY_SEPARATOR, QUOTE, SLICE_SEPARATOR, STATEMENT_SEPARATOR,
};
use greybel_symbols::literals::NULL;
use greybel_symbols::names::{BIT_AND, BIT_OR, BITWISE};
use greybel_symbols::{Keyword, Punct};

use super::nodes::{deferred_function, dependency_module};
use super::{EmitFlags, EmitOutput, EmitState, LiteralStyle};
use crate::bundle::{MULTILINE_STRING, REQUIRE};
use crate::error::{Result, TranspileError};

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

        match s.kind(id) {
            NodeKind::Chunk(chunk) => self.emit_body(s, s.ast().list(chunk.body)),
            NodeKind::Function(node) => self.emit_function(s, id, node, flags),
            NodeKind::Assignment(node) => self.emit_assignment(s, id, node, flags),
            NodeKind::CallStatement(node) => self.emit_call_statement(s, node, flags),
            NodeKind::Return(node) => self.emit_return(s, node),
            NodeKind::If(node) => self.emit_if(s, id, node),
            NodeKind::IfClause(node) => self.emit_if_clause(s, id, node),
            NodeKind::ElseClause(node) => self.emit_else_clause(s, id, node),
            NodeKind::While(node) => self.emit_while(s, node),
            NodeKind::For(node) => self.emit_for(s, node),
            NodeKind::Break(_) => self.emit_keyword(s, Keyword::Break),
            NodeKind::Continue(_) => self.emit_keyword(s, Keyword::Continue),
            NodeKind::Identifier(node) => self.emit_identifier(s, node, flags),
            NodeKind::Member(node) => self.emit_member(s, node),
            NodeKind::Index(node) => self.emit_index(s, node),
            NodeKind::Slice(node) => self.emit_slice(s, node),
            NodeKind::Call(node) => self.emit_call(s, id, node, flags),
            NodeKind::Literal(node) => self.emit_literal(s, id, node),
            NodeKind::Unary(node) => self.emit_unary(s, node),
            NodeKind::Binary(node) => self.emit_binary(s, node),
            NodeKind::Logical(node) => self.emit_logical(s, node, flags),
            NodeKind::Isa(node) => self.emit_isa(s, node, flags),
            NodeKind::ComparisonGroup(node) => self.emit_comparison_group(s, node),
            NodeKind::Map(node) => self.emit_map(s, id, node),
            NodeKind::MapEntry(node) => self.emit_map_entry(s, node, flags),
            NodeKind::List(node) => self.emit_list(s, id, node),
            NodeKind::ListItem(node) => self.emit_list_item(s, node, flags),
            NodeKind::Parenthesis(node) => self.emit_parenthesis(s, node, flags),
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
            if s.is_noop(item) {
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
            self.emit_body(s, s.ast().list(node.body))?;
            s.keyword(Keyword::EndFunction);
        }

        Ok(())
    }

    fn emit_function(
        &mut self,
        s: &mut EmitState<'_>,
        _id: NodeId,
        node: &Function,
        flags: EmitFlags,
    ) -> Result<()> {
        if node.parameters.is_empty() {
            s.keyword(Keyword::Function);
        } else {
            s.keyword(Keyword::Function);
            s.punct(Punct::LParen);
            self.emit_parameters(s, s.ast().list(node.parameters))?;
            s.punct(Punct::RParen);
        }

        if flags.header_only {
            return Ok(());
        }

        s.eol();
        self.emit_body(s, s.ast().list(node.body))?;
        s.keyword(Keyword::EndFunction);

        Ok(())
    }

    fn emit_parameters(&mut self, s: &mut EmitState<'_>, parameters: &[NodeId]) -> Result<()> {
        self.emit_with_separators(s, parameters, Punct::Comma.as_str())
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

    fn emit_if(&mut self, s: &mut EmitState<'_>, _id: NodeId, node: &If) -> Result<()> {
        if node.shortcut {
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

    fn emit_if_clause(
        &mut self,
        s: &mut EmitState<'_>,
        _id: NodeId,
        node: &IfClause,
    ) -> Result<()> {
        s.keyword(match node.kind {
            IfClauseKind::If => Keyword::If,
            IfClauseKind::ElseIf => Keyword::ElseIf,
        });
        s.push(" ");
        self.emit(s, node.condition, EmitFlags::default())?;
        if node.shortcut {
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
        _id: NodeId,
        node: &ElseClause,
    ) -> Result<()> {
        if node.shortcut {
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
        self.emit(s, node.key, EmitFlags::default())?;
        s.push(MAP_KEY_SEPARATOR);

        self.emit(s, node.value, flags)
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
            self.emit(s, node.name, EmitFlags::default())?;
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

        Ok(())
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
