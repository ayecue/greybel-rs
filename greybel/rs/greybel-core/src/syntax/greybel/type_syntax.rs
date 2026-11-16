use rustc_hash::FxHashMap;

use super::{AssertedType, TypeBody};
use crate::ast::{Ast, NodeId, NodeKind};
use crate::diagnostics::GreybelDiagnostic;
use crate::error::ParseResult;
use crate::lexer::{AnyToken, Invalid};
use crate::parser::Parser;
use crate::position::Span;

#[derive(Debug, Clone, Default)]
pub struct TypeSyntax {
    parameters: FxHashMap<NodeId, Span>,
    returns: FxHashMap<NodeId, Span>,
    templates: FxHashMap<NodeId, Vec<Span>>,
    variables: FxHashMap<NodeId, Span>,
    declarations: Vec<NodeId>,
}

impl TypeSyntax {
    #[must_use]
    pub fn parameter(&self, parameter: NodeId) -> Option<Span> {
        self.parameters.get(&parameter).copied()
    }

    #[must_use]
    pub fn returns(&self, function: NodeId) -> Option<Span> {
        self.returns.get(&function).copied()
    }

    #[must_use]
    pub fn templates(&self, function: NodeId) -> &[Span] {
        self.templates.get(&function).map_or(&[], Vec::as_slice)
    }

    #[must_use]
    pub fn variable(&self, name: NodeId) -> Option<Span> {
        self.variables.get(&name).copied()
    }

    #[must_use]
    pub fn declarations(&self) -> &[NodeId] {
        &self.declarations
    }

    pub fn spans(&self) -> impl Iterator<Item = Span> + '_ {
        self.parameters
            .values()
            .chain(self.returns.values())
            .chain(self.variables.values())
            .copied()
            .chain(self.templates.values().flatten().copied())
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.parameters.is_empty()
            && self.returns.is_empty()
            && self.templates.is_empty()
            && self.variables.is_empty()
            && self.declarations.is_empty()
    }

    pub(crate) fn add_declaration(&mut self, declaration: NodeId) {
        self.declarations.push(declaration);
    }

    pub(crate) fn set_parameter(&mut self, parameter: NodeId, ty: Span) {
        self.parameters.insert(parameter, ty);
    }

    pub(crate) fn set_returns(&mut self, function: NodeId, ty: Span) {
        self.returns.insert(function, ty);
    }

    pub(crate) fn set_templates(&mut self, function: NodeId, names: Vec<Span>) {
        self.templates.insert(function, names);
    }

    pub(crate) fn set_variable(&mut self, name: NodeId, ty: Span) {
        self.variables.insert(name, ty);
    }
}

pub(crate) struct TypeScan<'s> {
    source: &'s [u8],
    at: usize,
    depth: usize,
}

impl<'s> TypeScan<'s> {
    const MAX_DEPTH: usize = 32;

    pub(crate) fn new(source: &'s str, start: usize) -> Self {
        Self {
            source: source.as_bytes(),
            at: start,
            depth: 0,
        }
    }

    pub(crate) fn scan(mut self) -> Option<Span> {
        self.skip_spaces();

        let start = self.at;

        self.union()?;

        let start = u32::try_from(start).ok()?;
        let end = u32::try_from(self.at).ok()?;

        Some(Span::new(start, end))
    }

    fn union(&mut self) -> Option<()> {
        self.depth += 1;

        if self.depth > Self::MAX_DEPTH {
            return None;
        }

        self.primary()?;

        while self.consume(b"|") {
            self.primary()?;
        }

        self.depth -= 1;

        Some(())
    }

    fn primary(&mut self) -> Option<()> {
        self.skip_spaces();

        match self.source.get(self.at)? {
            b'(' => {
                self.parameters()?;

                if !self.consume(b"=>") {
                    return None;
                }

                self.union()
            }
            b'[' => {
                self.at += 1;

                self.list(b']')
            }
            b'"' => self.string(),
            byte if byte.is_ascii_digit() || *byte == b'-' => self.number(),
            _ => {
                let name = self.name()?;

                if name == b"function" && self.peek(b'(') {
                    self.parameters()?;

                    if self.consume(b":") {
                        self.union()?;
                    }

                    return Some(());
                }

                if self.consume(b"<") {
                    self.list(b'>')?;
                }

                let save = self.at;

                if self.word(b"is") {
                    return self.union();
                }

                self.at = save;

                Some(())
            }
        }
    }

    fn list(&mut self, close: u8) -> Option<()> {
        loop {
            self.union()?;

            if self.consume(&[close]) {
                return Some(());
            }

            if !self.consume(b",") {
                return None;
            }
        }
    }

    fn parameters(&mut self) -> Option<()> {
        if !self.consume(b"(") {
            return None;
        }

        if self.consume(b")") {
            return Some(());
        }

        loop {
            let save = self.at;

            if self.name().is_none() || !self.consume(b":") {
                self.at = save;
            }

            self.union()?;

            if self.consume(b")") {
                return Some(());
            }

            if !self.consume(b",") {
                return None;
            }
        }
    }

    fn name(&mut self) -> Option<&'s [u8]> {
        self.skip_spaces();

        let start = self.at;

        while self
            .source
            .get(self.at)
            .is_some_and(|&byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'.'))
        {
            self.at += 1;
        }

        (self.at > start).then(|| &self.source[start..self.at])
    }

    fn string(&mut self) -> Option<()> {
        self.at += 1;

        loop {
            match self.source.get(self.at)? {
                b'"' if self.source.get(self.at + 1) == Some(&b'"') => self.at += 2,
                b'"' => {
                    self.at += 1;

                    return Some(());
                }
                b'\n' => return None,
                _ => self.at += 1,
            }
        }
    }

    fn number(&mut self) -> Option<()> {
        let start = self.at;

        if self.source.get(self.at) == Some(&b'-') {
            self.at += 1;
        }

        while self
            .source
            .get(self.at)
            .is_some_and(|&byte| byte.is_ascii_digit() || byte == b'.')
        {
            self.at += 1;
        }

        (self.at > start + 1 || self.source[start].is_ascii_digit()).then_some(())
    }

    fn word(&mut self, word: &[u8]) -> bool {
        let save = self.at;

        self.skip_spaces();

        let ends = self
            .source
            .get(self.at + word.len())
            .is_none_or(|&byte| !(byte.is_ascii_alphanumeric() || byte == b'_'));

        if self.at > save && self.source[self.at..].starts_with(word) && ends {
            self.at += word.len();

            return true;
        }

        self.at = save;

        false
    }

    fn peek(&mut self, byte: u8) -> bool {
        let save = self.at;

        self.skip_spaces();

        let found = self.source.get(self.at) == Some(&byte);

        self.at = save;

        found
    }

    fn consume(&mut self, text: &[u8]) -> bool {
        let save = self.at;

        self.skip_spaces();

        if self.source[self.at..].starts_with(text) {
            self.at += text.len();

            return true;
        }

        self.at = save;

        false
    }

    fn skip_spaces(&mut self) {
        while matches!(self.source.get(self.at), Some(b' ' | b'\t')) {
            self.at += 1;
        }
    }
}

impl Parser<'_> {
    pub(crate) fn parse_type_text(&mut self, start: u32) -> ParseResult<Option<Span>> {
        let Some(span) = TypeScan::new(self.source(), start as usize).scan() else {
            let span = self.token().span();

            self.raise_at(GreybelDiagnostic::ExpectedType, span)?;

            return Ok(None);
        };

        self.stream.tokens.cursor.set(span.end as usize);
        self.stream
            .tokens
            .lookahead
            .push(AnyToken::Invalid(Invalid::new(span)));
        self.advance()?;

        Ok(Some(span))
    }
}

impl Ast {
    #[must_use]
    pub fn type_spans(&self) -> Vec<Span> {
        let mut spans: Vec<Span> = self.type_syntax.spans().collect();

        for (_, node) in self.nodes().iter() {
            match &node.kind {
                NodeKind::TypeStatement(statement) => {
                    spans.push(statement.name);
                    spans.extend(statement.type_parameters.iter().copied());

                    match &statement.body {
                        TypeBody::Alias(ty) => spans.push(*ty),
                        TypeBody::Members { extends, members } => {
                            spans.extend(extends.iter().copied());

                            for member in members {
                                spans.extend(member.ty.iter().copied());
                                spans.extend(
                                    member.parameters.iter().flatten().filter_map(|&(_, ty)| ty),
                                );
                            }
                        }
                    }
                }
                NodeKind::TypeAssertion(assertion) => {
                    if let AssertedType::Type(ty) = assertion.target {
                        spans.push(ty);
                    }
                }
                NodeKind::Class(class) => spans.extend(class.implements.iter().copied()),
                _ => {}
            }
        }

        spans
    }
}
