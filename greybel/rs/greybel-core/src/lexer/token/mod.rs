mod boolean;
mod comment;
mod eof;
mod eol;
mod identifier;
mod invalid;
pub mod keyword;
mod nil;
mod number;
mod pending_token;
pub mod punct;
mod slice;
mod string;

use std::borrow::Cow;
use std::fmt;

pub use boolean::BooleanLiteral;
pub use comment::Comment;
pub use eof::Eof;
pub use eol::Eol;
pub use identifier::Identifier;
pub use invalid::Invalid;
pub use keyword::{KEYWORDS, Keyword, KeywordToken};
pub use nil::NilLiteral;
pub use number::NumericLiteral;
pub use pending_token::PendingToken;
pub use punct::{PUNCTUATORS, Punct, Punctuator};
pub use slice::SliceOperator;
pub use string::{
    ESCAPED_CLOSE_BRACE, ESCAPED_OPEN_BRACE, StringLiteral, TEMPLATE_START, TemplateScan,
    TemplateShape,
};

use crate::position::{ColumnSettings, LineIndex, Span};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TokenKind {
    Eof,
    Eol,
    StringLiteral,
    NumericLiteral,
    BooleanLiteral,
    NilLiteral,
    Keyword,
    Identifier,
    Punctuator,
    SliceOperator,
    Comment,
    Invalid,
}

impl TokenKind {
    #[must_use]
    pub const fn name(self) -> &'static str {
        match self {
            TokenKind::Eof => "EOF",
            TokenKind::Eol => "EOL",
            TokenKind::StringLiteral => "StringLiteral",
            TokenKind::NumericLiteral => "NumericLiteral",
            TokenKind::BooleanLiteral => "BooleanLiteral",
            TokenKind::NilLiteral => "NilLiteral",
            TokenKind::Keyword => "Keyword",
            TokenKind::Identifier => "Identifier",
            TokenKind::Punctuator => "Punctuator",
            TokenKind::SliceOperator => "SliceOperator",
            TokenKind::Comment => "Comment",
            TokenKind::Invalid => "Invalid",
        }
    }

    #[must_use]
    pub const fn is_literal(self) -> bool {
        matches!(
            self,
            TokenKind::StringLiteral
                | TokenKind::NumericLiteral
                | TokenKind::BooleanLiteral
                | TokenKind::NilLiteral
        )
    }
}

pub trait Token: fmt::Debug {
    fn kind(&self) -> TokenKind;
    fn span(&self) -> Span;
    fn has_whitespace_before(&self) -> bool;
    fn value<'s>(&self, source: &'s str) -> Cow<'s, str>;
    fn describe(&self, source: &str) -> String;

    fn display_value<'s>(&self, source: &'s str) -> Cow<'s, str> {
        Cow::Owned(format!("'{}'", self.value(source)))
    }

    fn keyword(&self) -> Option<Keyword> {
        None
    }

    fn punct(&self) -> Option<Punct> {
        None
    }

    fn as_string(&self) -> Option<&StringLiteral> {
        None
    }

    fn as_number(&self) -> Option<&NumericLiteral> {
        None
    }

    fn as_boolean(&self) -> Option<&BooleanLiteral> {
        None
    }

    fn as_identifier(&self) -> Option<&Identifier> {
        None
    }

    fn as_comment(&self) -> Option<&Comment> {
        None
    }

    fn type_name(&self) -> &'static str {
        self.kind().name()
    }

    fn is_literal(&self) -> bool {
        self.kind().is_literal()
    }

    fn is_eof(&self) -> bool {
        self.kind() == TokenKind::Eof
    }

    fn is_eol(&self) -> bool {
        self.kind() == TokenKind::Eol
    }

    fn is_comment(&self) -> bool {
        self.kind() == TokenKind::Comment
    }

    fn is_identifier(&self) -> bool {
        self.kind() == TokenKind::Identifier
    }

    fn is_keyword(&self, keyword: Keyword) -> bool {
        self.keyword() == Some(keyword)
    }

    fn is_punct(&self, punct: Punct) -> bool {
        self.punct() == Some(punct)
    }
}

#[derive(Debug, Clone, Copy)]
pub enum AnyToken {
    Eof(Eof),
    Eol(Eol),
    StringLiteral(StringLiteral),
    NumericLiteral(NumericLiteral),
    BooleanLiteral(BooleanLiteral),
    NilLiteral(NilLiteral),
    Keyword(KeywordToken),
    Identifier(Identifier),
    Punctuator(Punctuator),
    SliceOperator(SliceOperator),
    Comment(Comment),
    Invalid(Invalid),
}

impl AnyToken {
    #[must_use]
    pub fn as_token(&self) -> &(dyn Token + 'static) {
        match self {
            AnyToken::Eof(token) => token,
            AnyToken::Eol(token) => token,
            AnyToken::StringLiteral(token) => token,
            AnyToken::NumericLiteral(token) => token,
            AnyToken::BooleanLiteral(token) => token,
            AnyToken::NilLiteral(token) => token,
            AnyToken::Keyword(token) => token,
            AnyToken::Identifier(token) => token,
            AnyToken::Punctuator(token) => token,
            AnyToken::SliceOperator(token) => token,
            AnyToken::Comment(token) => token,
            AnyToken::Invalid(token) => token,
        }
    }
}

impl AnyToken {
    #[must_use]
    pub fn kind(&self) -> TokenKind {
        match self {
            AnyToken::Eof(_) => TokenKind::Eof,
            AnyToken::Eol(_) => TokenKind::Eol,
            AnyToken::StringLiteral(_) => TokenKind::StringLiteral,
            AnyToken::NumericLiteral(_) => TokenKind::NumericLiteral,
            AnyToken::BooleanLiteral(_) => TokenKind::BooleanLiteral,
            AnyToken::NilLiteral(_) => TokenKind::NilLiteral,
            AnyToken::Keyword(_) => TokenKind::Keyword,
            AnyToken::Identifier(_) => TokenKind::Identifier,
            AnyToken::Punctuator(_) => TokenKind::Punctuator,
            AnyToken::SliceOperator(_) => TokenKind::SliceOperator,
            AnyToken::Comment(_) => TokenKind::Comment,
            AnyToken::Invalid(_) => TokenKind::Invalid,
        }
    }

    #[must_use]
    pub fn span(&self) -> Span {
        match self {
            AnyToken::Eof(token) => token.span,
            AnyToken::Eol(token) => token.span,
            AnyToken::StringLiteral(token) => token.span,
            AnyToken::NumericLiteral(token) => token.span,
            AnyToken::BooleanLiteral(token) => token.span,
            AnyToken::NilLiteral(token) => token.span,
            AnyToken::Keyword(token) => token.span,
            AnyToken::Identifier(token) => token.span,
            AnyToken::Punctuator(token) => token.span,
            AnyToken::SliceOperator(token) => token.span,
            AnyToken::Comment(token) => token.span,
            AnyToken::Invalid(token) => token.span,
        }
    }

    #[must_use]
    pub fn keyword(&self) -> Option<Keyword> {
        match self {
            AnyToken::Keyword(token) => Some(token.keyword),
            _ => None,
        }
    }

    #[must_use]
    pub fn punct(&self) -> Option<Punct> {
        match self {
            AnyToken::Punctuator(token) => Some(token.punct),
            _ => None,
        }
    }

    #[must_use]
    pub fn is_keyword(&self, keyword: Keyword) -> bool {
        matches!(self, AnyToken::Keyword(token) if token.keyword == keyword)
    }

    #[must_use]
    pub fn is_punct(&self, punct: Punct) -> bool {
        matches!(self, AnyToken::Punctuator(token) if token.punct == punct)
    }

    #[must_use]
    pub fn is_eof(&self) -> bool {
        matches!(self, AnyToken::Eof(_))
    }

    #[must_use]
    pub fn is_eol(&self) -> bool {
        matches!(self, AnyToken::Eol(_))
    }

    #[must_use]
    pub fn is_comment(&self) -> bool {
        matches!(self, AnyToken::Comment(_))
    }

    #[must_use]
    pub fn is_identifier(&self) -> bool {
        matches!(self, AnyToken::Identifier(_))
    }

    #[must_use]
    pub fn is_literal(&self) -> bool {
        matches!(
            self,
            AnyToken::StringLiteral(_)
                | AnyToken::NumericLiteral(_)
                | AnyToken::BooleanLiteral(_)
                | AnyToken::NilLiteral(_)
        )
    }

    /// A plain string literal; `#"..."` templates are [`Self::as_template`].
    #[must_use]
    pub fn as_string(&self) -> Option<&StringLiteral> {
        match self {
            AnyToken::StringLiteral(token) if !token.template => Some(token),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_template(&self) -> Option<&StringLiteral> {
        match self {
            AnyToken::StringLiteral(token) if token.template => Some(token),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_number(&self) -> Option<&NumericLiteral> {
        match self {
            AnyToken::NumericLiteral(token) => Some(token),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_boolean(&self) -> Option<&BooleanLiteral> {
        match self {
            AnyToken::BooleanLiteral(token) => Some(token),
            _ => None,
        }
    }

    #[must_use]
    pub fn as_identifier(&self) -> Option<&Identifier> {
        match self {
            AnyToken::Identifier(token) => Some(token),
            _ => None,
        }
    }
}

impl std::ops::Deref for AnyToken {
    type Target = dyn Token;

    fn deref(&self) -> &Self::Target {
        self.as_token()
    }
}

impl From<Eof> for AnyToken {
    fn from(token: Eof) -> Self {
        AnyToken::Eof(token)
    }
}

impl From<Eol> for AnyToken {
    fn from(token: Eol) -> Self {
        AnyToken::Eol(token)
    }
}

impl From<StringLiteral> for AnyToken {
    fn from(token: StringLiteral) -> Self {
        AnyToken::StringLiteral(token)
    }
}

impl From<NumericLiteral> for AnyToken {
    fn from(token: NumericLiteral) -> Self {
        AnyToken::NumericLiteral(token)
    }
}

impl From<BooleanLiteral> for AnyToken {
    fn from(token: BooleanLiteral) -> Self {
        AnyToken::BooleanLiteral(token)
    }
}

impl From<NilLiteral> for AnyToken {
    fn from(token: NilLiteral) -> Self {
        AnyToken::NilLiteral(token)
    }
}

impl From<KeywordToken> for AnyToken {
    fn from(token: KeywordToken) -> Self {
        AnyToken::Keyword(token)
    }
}

impl From<Identifier> for AnyToken {
    fn from(token: Identifier) -> Self {
        AnyToken::Identifier(token)
    }
}

impl From<Punctuator> for AnyToken {
    fn from(token: Punctuator) -> Self {
        AnyToken::Punctuator(token)
    }
}

impl From<SliceOperator> for AnyToken {
    fn from(token: SliceOperator) -> Self {
        AnyToken::SliceOperator(token)
    }
}

impl From<Comment> for AnyToken {
    fn from(token: Comment) -> Self {
        AnyToken::Comment(token)
    }
}

impl From<Invalid> for AnyToken {
    fn from(token: Invalid) -> Self {
        AnyToken::Invalid(token)
    }
}

impl dyn Token + '_ {
    pub fn display<'a>(
        &'a self,
        source: &'a str,
        lines: &'a LineIndex,
        columns: ColumnSettings,
    ) -> impl fmt::Display + 'a {
        TokenDisplay {
            token: self,
            source,
            lines,
            columns,
        }
    }
}

struct TokenDisplay<'a> {
    token: &'a (dyn Token + 'a),
    source: &'a str,
    lines: &'a LineIndex,
    columns: ColumnSettings,
}

impl fmt::Display for TokenDisplay<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let token = self.token;
        let location = self.lines.range(token.span(), self.columns);

        write!(
            f,
            "{}[{location}: value = {}]",
            token.type_name(),
            token.display_value(self.source)
        )
    }
}
