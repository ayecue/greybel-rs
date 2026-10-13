use crate::diagnostics::KeywordDiagnostic;
use crate::error::ParseResult;
use crate::lexer::Keyword;
use crate::parser::{KeywordStatement, Parser};

#[derive(Debug, Clone, Copy)]
pub struct MisplacedKeywordRule {
    pub keyword: Keyword,
}

impl MisplacedKeywordRule {
    pub const AND: Self = Self {
        keyword: Keyword::And,
    };
    pub const FUNCTION: Self = Self {
        keyword: Keyword::Function,
    };
    pub const IN: Self = Self {
        keyword: Keyword::In,
    };
    pub const ISA: Self = Self {
        keyword: Keyword::Isa,
    };
    pub const NEW: Self = Self {
        keyword: Keyword::New,
    };
    pub const OR: Self = Self {
        keyword: Keyword::Or,
    };
    pub const REPEAT: Self = Self {
        keyword: Keyword::Repeat,
    };
    pub const THEN: Self = Self {
        keyword: Keyword::Then,
    };
}

impl KeywordStatement for MisplacedKeywordRule {
    fn keywords(&self) -> &[Keyword] {
        std::slice::from_ref(&self.keyword)
    }

    fn parse(&self, parser: &mut Parser<'_>, _keyword: Keyword) -> ParseResult<()> {
        let span = parser.token().span();

        parser.raise_at(KeywordDiagnostic::Misplacement(self.keyword), span)
    }
}
