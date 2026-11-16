use std::borrow::Cow;

pub use greybel_symbols::Punct;

use super::{AnyToken, PendingToken, Token, TokenKind};
use crate::lexer::{Lexer, TokenStream};
use crate::position::Span;

pub const PUNCTUATORS: &[Punct] = &[
    Punct::Plus,
    Punct::Minus,
    Punct::Asterisk,
    Punct::Slash,
    Punct::Caret,
    Punct::Percent,
    Punct::Less,
    Punct::Greater,
    Punct::LessEqual,
    Punct::GreaterEqual,
    Punct::NotEqual,
    Punct::Equal,
    Punct::Dot,
    Punct::Assign,
    Punct::Comma,
    Punct::LBracket,
    Punct::RBracket,
    Punct::LBrace,
    Punct::RBrace,
    Punct::LParen,
    Punct::RParen,
    Punct::At,
];

pub const COMPOUND_ASSIGNMENTS: &[Punct] = &[
    Punct::PlusAssign,
    Punct::MinusAssign,
    Punct::AsteriskAssign,
    Punct::SlashAssign,
    Punct::CaretAssign,
    Punct::PercentAssign,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Punctuator {
    pub(super) span: Span,
    pub(super) after_space: bool,
    pub(super) punct: Punct,
}

impl Punctuator {
    #[must_use]
    pub fn describe_punct(punct: Punct) -> String {
        let name = match punct {
            Punct::Plus => "OpPlus",
            Punct::Minus => "OpMinus",
            Punct::Asterisk => "OpTimes",
            Punct::Slash => "OpDivide",
            Punct::Caret => "OpPower",
            Punct::Percent => "OpMod",
            Punct::Less => "OpLesser",
            Punct::Greater => "OpGreater",
            Punct::LessEqual => "OpLessEqual",
            Punct::GreaterEqual => "OpGreatEqual",
            Punct::NotEqual => "OpNotEqual",
            Punct::Equal => "OpEqual",
            Punct::Dot => "Dot",
            Punct::Assign => "OpAssign",
            Punct::Comma => "Comma",
            Punct::LBracket => "LSquare",
            Punct::RBracket => "RSquare",
            Punct::LBrace => "LCurly",
            Punct::RBrace => "RCurly",
            Punct::LParen => "LParen",
            Punct::RParen => "RParen",
            Punct::At => "AddressOf",
            _ => return format!("Punctuator({})", punct.as_str()),
        };

        name.to_owned()
    }
}

impl Token for Punctuator {
    fn kind(&self) -> TokenKind {
        TokenKind::Punctuator
    }

    fn span(&self) -> Span {
        self.span
    }

    fn has_whitespace_before(&self) -> bool {
        self.after_space
    }

    fn value<'s>(&self, _source: &'s str) -> Cow<'s, str> {
        Cow::Borrowed(self.punct.as_str())
    }

    fn describe(&self, _source: &str) -> String {
        Self::describe_punct(self.punct)
    }

    fn punct(&self) -> Option<Punct> {
        Some(self.punct)
    }
}

impl<'src> Lexer<'src> {
    pub fn scan_punct(
        &self,
        stream: &mut TokenStream,
        punct: Punct,
        token_start: PendingToken,
    ) -> AnyToken {
        stream.cursor.advance(punct.as_str().len());

        token_start.finish(stream, |span, after_space| Punctuator {
            span,
            after_space,
            punct,
        })
    }
}
