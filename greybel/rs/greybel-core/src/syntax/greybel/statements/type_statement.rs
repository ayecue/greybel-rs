use greybel_symbols::contextual_keywords::{EXTENDS, READONLY};

use crate::ast::{AstNode, DumpContext, NodeFieldContainer, NodeId, TextSpan};
use crate::diagnostics::GreybelDiagnostic;
use crate::error::ParseResult;
use crate::lexer::{Keyword, Punct, TokenKind};
use crate::parser::{Expect, Parser};
use crate::position::Span;

/// `type Name<T> = T` (an alias) or `type Name<T> extends Base` with members up to `end type` (a type without runtime
/// code, like `@vtype`). Builds write nothing for it.
#[derive(Debug, Clone, PartialEq, NodeFieldContainer)]
pub struct TypeStatement {
    pub name: Span,
    pub type_parameters: Vec<Span>,
    pub body: TypeBody,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypeBody {
    Alias(Span),
    Members {
        extends: Option<Span>,
        members: Vec<TypeMember>,
    },
}

/// `[readonly] name[?]: T` or `name(parameter: T, ...): T` in a `type` body.
#[derive(Debug, Clone, PartialEq)]
pub struct TypeMember {
    pub name: Span,
    /// The member's type, or a method's return type.
    pub ty: Option<Span>,
    pub optional: bool,
    pub readonly: bool,
    /// A method's parameters: each name with its type.
    pub parameters: Option<Vec<(Span, Option<Span>)>>,
}

impl AstNode for TypeStatement {
    fn type_name(&self) -> &'static str {
        "TypeStatement"
    }

    fn dump(&self, ctx: &DumpContext<'_>) -> String {
        let name = ctx.read(TextSpan::new(self.name));

        match &self.body {
            TypeBody::Alias(ty) => format!(
                "TypeStatement[{}][{name} = {}]",
                ctx.location(),
                ctx.read(TextSpan::new(*ty))
            ),
            TypeBody::Members { members, .. } => format!(
                "TypeStatement[{}][{name} with {} members]",
                ctx.location(),
                members.len()
            ),
        }
    }
}

impl TypeStatement {
    pub(crate) fn parse(parser: &mut Parser<'_>) -> ParseResult<NodeId> {
        let start = parser.token().span();

        parser.advance()?;

        let name = parser.token().span();

        if parser.token().as_identifier().is_none() {
            parser.raise_at(GreybelDiagnostic::ExpectedType, name)?;
        }

        parser.advance()?;

        let type_parameters = Self::parse_type_parameters(parser)?;
        let body = if parser.token().is_punct(Punct::Assign) {
            let type_start = parser.token().span().end;
            let ty = parser.parse_type_text(type_start)?.unwrap_or(name);

            TypeBody::Alias(ty)
        } else {
            Self::parse_members(parser, start)?
        };
        let span = match body {
            TypeBody::Alias(ty) => Span::new(start.start, ty.end),
            TypeBody::Members { .. } => parser.span_from(&start),
        };
        let node = parser.alloc(
            TypeStatement {
                name,
                type_parameters,
                body,
            },
            span,
        );

        parser.stream.type_syntax.add_declaration(node);

        Ok(node)
    }

    fn parse_type_parameters(parser: &mut Parser<'_>) -> ParseResult<Vec<Span>> {
        let mut names = Vec::new();

        if !parser.consume(Expect::Punct(Punct::Less))? {
            return Ok(names);
        }

        loop {
            if parser.token().as_identifier().is_none() {
                let span = parser.token().span();

                parser.raise_at(GreybelDiagnostic::ExpectedType, span)?;

                return Ok(names);
            }

            names.push(parser.token().span());
            parser.advance()?;

            if !parser.consume(Expect::Punct(Punct::Comma))? {
                break;
            }
        }

        parser.require(Expect::Punct(Punct::Greater), None)?;

        Ok(names)
    }

    /// `[extends Base]`, then a member per line up to `end type`.
    fn parse_members(parser: &mut Parser<'_>, start: Span) -> ParseResult<TypeBody> {
        let extends = if Self::is_at_word(parser, EXTENDS) {
            let type_start = parser.token().span().end;

            parser.parse_type_text(type_start)?
        } else {
            None
        };
        let mut members = Vec::new();

        loop {
            while parser.token().is_eol() || parser.token().is_comment() {
                parser.advance()?;
            }

            if parser.token().is_eof() {
                parser.raise_at(GreybelDiagnostic::UnclosedType, start)?;

                break;
            }

            if parser.token().is_keyword(Keyword::EndType) {
                parser.advance()?;

                break;
            }

            match Self::parse_member(parser)? {
                Some(member) => members.push(member),
                None => Self::skip_line(parser)?,
            }
        }

        Ok(TypeBody::Members { extends, members })
    }

    fn parse_member(parser: &mut Parser<'_>) -> ParseResult<Option<TypeMember>> {
        let readonly = Self::is_at_word(parser, READONLY) && Self::is_followed_by_name(parser);

        if readonly {
            parser.advance()?;
        }

        let name = parser.token().span();

        if parser.token().as_identifier().is_none() {
            parser.raise_at(GreybelDiagnostic::TypeMember, name)?;

            return Ok(None);
        }

        parser.advance()?;

        let optional = parser.consume(Expect::Punct(Punct::Question))?;
        let parameters = if parser.token().is_punct(Punct::LParen) {
            Some(Self::parse_parameters(parser)?)
        } else {
            None
        };
        let ty = if parser.token().kind() == TokenKind::SliceOperator {
            let type_start = parser.token().span().end;

            parser.parse_type_text(type_start)?
        } else {
            if parameters.is_none() {
                let span = parser.token().span();

                parser.raise_at(GreybelDiagnostic::TypeMember, span)?;

                return Ok(None);
            }

            None
        };

        Ok(Some(TypeMember {
            name,
            ty,
            optional,
            readonly,
            parameters,
        }))
    }

    /// `(name: T, name, ...)`.
    fn parse_parameters(parser: &mut Parser<'_>) -> ParseResult<Vec<(Span, Option<Span>)>> {
        parser.advance()?;

        let mut parameters = Vec::new();

        while !parser.token().is_punct(Punct::RParen) {
            let name = parser.token().span();

            if parser.token().as_identifier().is_none() {
                parser.raise_at(GreybelDiagnostic::TypeMember, name)?;

                return Ok(parameters);
            }

            parser.advance()?;

            let ty = if parser.token().kind() == TokenKind::SliceOperator {
                let type_start = parser.token().span().end;

                parser.parse_type_text(type_start)?
            } else {
                None
            };

            parameters.push((name, ty));

            if !parser.consume(Expect::Punct(Punct::Comma))? {
                break;
            }
        }

        parser.require(Expect::Punct(Punct::RParen), None)?;

        Ok(parameters)
    }

    fn skip_line(parser: &mut Parser<'_>) -> ParseResult<()> {
        while !(parser.token().is_eol() || parser.token().is_eof()) {
            parser.advance()?;
        }

        Ok(())
    }

    fn is_at_word(parser: &Parser<'_>, word: &str) -> bool {
        parser
            .token()
            .as_identifier()
            .is_some_and(|identifier| identifier.name(parser.source()) == word)
    }

    fn is_followed_by_name(parser: &Parser<'_>) -> bool {
        let span = parser.token().span();
        let after = parser.source()[span.end as usize..].trim_start_matches([' ', '\t']);

        after.len() < parser.source().len() - span.end as usize
            && after
                .starts_with(|character: char| character.is_ascii_alphabetic() || character == '_')
    }
}
