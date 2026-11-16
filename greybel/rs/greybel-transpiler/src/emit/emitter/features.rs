use greybel_core::ast::NodeList;
use greybel_core::syntax::Literal;
use greybel_core::syntax::greybel::{
    Assert, AssertedType, Class, ClassMember, Enum, SwitchCase, Template, TypeAssertion,
};
use greybel_core::{ColumnSettings, LiteralValue, NodeId, NodeKind};
use greybel_symbols::contextual_keywords::{
    AS, ASSERT, CASE, CLASS, EXPORT, EXTENDS, IMPLEMENTS, IS, PRIVATE, STATIC,
};
use greybel_symbols::delimiters::{ESCAPED_QUOTE, QUOTE, SLICE_SEPARATOR};
use greybel_symbols::names::EXIT;
use greybel_symbols::{Keyword, Punct};

use super::{EmitExpressions, EmitFunctions, EmitStatements, Emitter};
use crate::emit::{EmitFlags, EmitState};
use crate::error::{Result, TranspileError};

const LINE_BREAK_CHARACTER: &str = "char(10)";

pub(crate) trait EmitFeatures: Emitter {
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

                if s.is_operand(argument) {
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

    fn emit_type_assertion(
        &mut self,
        s: &mut EmitState<'_>,
        node: &TypeAssertion,
        flags: EmitFlags,
    ) -> Result<()> {
        self.emit(s, node.expression, flags)?;

        if s.options().dev_mode {
            match node.target {
                AssertedType::NonNull => s.push(Punct::Exist.as_str()),
                AssertedType::Type(span) => {
                    let ty = s.ast().slice(span).to_owned();

                    s.push_fmt(format_args!(" {AS} {ty}"));
                }
            }
        }

        Ok(())
    }

    fn emit_written_type(&mut self, s: &mut EmitState<'_>, id: NodeId) {
        if let Some(ty) = s.written_type(id) {
            s.push_fmt(format_args!(": {ty}"));
        }
    }

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
            let parameters = s.emitted_parameters(s.ast().list(function.parameters));

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
                s.push_fmt(format_args!("{padding}{}{padding}", Punct::Conditional));
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
}

impl<E: Emitter> EmitFeatures for E {}
