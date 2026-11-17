mod directives;
mod expressions;
mod features;
mod functions;
mod statements;

use greybel_core::syntax::greybel::{Include, Inject, Switch};
use greybel_core::syntax::{
    Assignment, Binary, Call, CallStatement, ComparisonGroup, ElseClause, For, Function,
    Identifier, If, IfClause, Isa, List, ListItem, Literal, Logical, Map, MapEntry, Member,
    Parenthesis, Slice, While,
};
use greybel_core::{IfClauseKind, NodeId, NodeKind};
use greybel_symbols::contextual_keywords::{SWITCH, TYPE};
use greybel_symbols::delimiters::{
    ESCAPED_QUOTE, LINE_COMMENT, MAP_KEY_SEPARATOR, QUOTE, SLICE_SEPARATOR, STATEMENT_SEPARATOR,
};
use greybel_symbols::literals::NULL;
use greybel_symbols::{Keyword, Punct};

pub(crate) use self::directives::EmitDirectives;
pub(crate) use self::expressions::EmitExpressions;
pub(crate) use self::features::EmitFeatures;
pub(crate) use self::functions::EmitFunctions;
pub(crate) use self::statements::EmitStatements;
use super::{EmitFlags, EmitOutput, EmitState, LiteralStyle};
use crate::bundle::{REQUIRE, multiline_string_helper};
use crate::error::Result;

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
            NodeKind::Return(_) if s.is_self_return(id) => {
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
            NodeKind::Map(node) if !s.options().dev_mode && s.has_spread(node.fields) => {
                self.emit_spread_collection(s, node.fields, (Punct::LBrace, Punct::RBrace))
            }
            NodeKind::List(node) if !s.options().dev_mode && s.has_spread(node.fields) => {
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
        let parameters = s.emitted_parameters(parameters);

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

    fn emit_member(&mut self, s: &mut EmitState<'_>, node: &Member) -> Result<()> {
        self.emit(s, node.base, EmitFlags::default())?;
        s.punct(Punct::Dot);

        let identifier = EmitFlags {
            is_member: true,
            ..EmitFlags::default()
        };

        self.emit(s, node.identifier, identifier)
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

    fn emit_literal(&mut self, s: &mut EmitState<'_>, id: NodeId, node: &Literal) -> Result<()> {
        s.literal(
            id,
            node,
            s.text(id),
            LiteralStyle::Output,
            self.runtime_name(multiline_string_helper(s.options())),
        );

        Ok(())
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

    fn separate_statements(&mut self, s: &mut EmitState<'_>) {
        s.eol();
    }

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

    fn emit_arrow_body(&mut self, s: &mut EmitState<'_>, node: &Function) -> Result<()> {
        s.eol();
        self.emit_function_body(s, node)?;
        s.keyword(Keyword::EndFunction);

        Ok(())
    }

    fn emit_block_source(&mut self, s: &mut EmitState<'_>, id: NodeId) {
        let text = s.text(id).to_owned();

        s.push(&text);
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
}
